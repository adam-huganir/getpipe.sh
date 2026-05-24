use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

use crate::domain::platform::{TargetArch, TargetOs};
use crate::error::AppError;
use crate::http::query::{InstallMethod, InstallQueryOptions};
use crate::http::responses::ScriptResponse;
use crate::services::installer;
use crate::supported_apps::{self, Repo, SupportedApp};
use crossterm::{
  execute,
  style::{style, Color, Print, Stylize},
};
use flate2::read::GzDecoder;
use mime::Mime;
use serde_json::Value;
use std::env;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tar::Archive;
use zip::ZipArchive;

#[derive(Parser)]
#[command(name = "termlibs")]
#[command(about = "Termlibs server and install script CLI")]
#[command(version)]
pub(crate) struct Cli {
  #[command(subcommand)]
  pub(crate) command: Option<Commands>,
}

#[derive(Args, Debug)]
pub(crate) struct CompletionsArgs {
  /// Shell to generate completions for
  #[arg(value_enum)]
  pub(crate) shell: Shell,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
  /// Run the HTTP server (default when no command is provided)
  Serve(ServeArgs),
  /// Script-focused commands (templated installers)
  #[command(subcommand)]
  Script(ScriptCommands),
  /// Native installer (placeholder for Rust implementation)
  Install(InstallArgs),
  /// Generate shell completion scripts
  Completions(CompletionsArgs),
}

#[derive(Args, Debug, Default)]
pub(crate) struct ServeArgs {
  /// Port to bind the HTTP server
  #[arg(long)]
  port: Option<u16>,
  /// Listen address for the HTTP server
  #[arg(long)]
  listen: Option<String>,
  /// Logging level (DEBUG, INFO, WARN, ERROR)
  #[arg(long)]
  log_level: Option<String>,
  /// Enable request logging middleware
  #[arg(long)]
  log_requests: bool,
}

impl ServeArgs {
  pub(crate) fn port(&self) -> Option<u16> {
    self.port
  }

  pub(crate) fn listen(&self) -> Option<&str> {
    self.listen.as_deref()
  }

  pub(crate) fn log_level(&self) -> Option<&str> {
    self.log_level.as_deref()
  }

  pub(crate) fn log_requests(&self) -> bool {
    self.log_requests
  }
}

#[derive(Args, Debug)]
pub(crate) struct InstallArgs {
  /// Target to install: <app> or <owner> <repo>
  #[arg(value_name = "APP|OWNER REPO", num_args = 1..=2)]
  target: Vec<String>,
  /// Target operating system
  #[arg(long)]
  os: Option<String>,
  /// Target architecture
  #[arg(long)]
  arch: Option<String>,
  /// Release version or tag (default: latest)
  #[arg(long)]
  version: Option<String>,
  /// Installation prefix (default: $HOME/.local)
  #[arg(long)]
  prefix: Option<String>,
  /// Install method hint: binary or installer
  #[arg(long)]
  method: Option<String>,
  /// Download-only mode
  #[arg(long)]
  download_only: bool,
  /// Force installation
  #[arg(long)]
  force: bool,
  /// Quiet mode
  #[arg(long)]
  quiet: bool,
  /// Log level injected into the script
  #[arg(long)]
  log_level: Option<String>,
}

impl InstallArgs {
  pub(crate) async fn run(&self) -> Result<CliInstallOutput, AppError> {
    let plan = NativeInstallPlan::from_args(self)?;
    let tempdir = create_tempdir()?;
    let (links, labels) = plan.resolve_links().await?;
    let selection = if labels.len() == 1 {
      ct_write_line(
        style(format!("Only one download found; selecting {}", labels[0])).with(Color::Yellow),
      )?;
      0
    } else {
      prompt_for_choice(&labels)?
    };
    let selected_link = links
      .get(selection)
      .ok_or_else(|| AppError::InvalidInput("selection out of bounds".to_string()))?;

    let mut chosen_path = None;
    let mut chosen_name = None;
    let mut chosen_archive_entry = None;
    if is_probably_binary(&selected_link.content_type) {
      let downloaded_path =
        download_asset(&selected_link.url, &tempdir, &selected_link.name).await?;
      let default_dir = env::current_dir()
        .map(|p| p.join("bin"))
        .unwrap_or_else(|_| PathBuf::from("bin"));
      let default_name = default_binary_name(&plan.target, selected_link);
      let (dest_dir, binary_name) =
        prompt_destination(&selected_link.name, &default_dir, &default_name)?;
      let final_path = finalize_install(&downloaded_path, &dest_dir, &binary_name)?;
      chosen_path = Some(dest_dir);
      chosen_name = Some(binary_name);
      ct_write_line(style(format!("Copied to {}", final_path.display())).with(Color::Green))?;
    } else if is_archive(&selected_link.content_type, &selected_link.name) {
      let downloaded_path =
        download_asset(&selected_link.url, &tempdir, &selected_link.name).await?;
      let entries = list_archive_entries(&downloaded_path)?;
      if entries.is_empty() {
        return Err(AppError::InvalidInput("archive is empty".to_string()));
      }
      if entries.len() == 1 {
        ct_write_line(
          style(format!("Only one entry found; selecting {}", entries[0])).with(Color::Yellow),
        )?;
        chosen_archive_entry = Some(entries[0].clone());
      } else {
        render_tree(&entries)?;
        let archive_choice = prompt_for_choice(&entries)?;
        chosen_archive_entry = Some(entries[archive_choice].clone());
      }

      if let Some(entry) = &chosen_archive_entry {
        let extracted = extract_archive_entry(&downloaded_path, entry, &tempdir)?;
        let default_dir = env::current_dir()
          .map(|p| p.join("bin"))
          .unwrap_or_else(|_| PathBuf::from("bin"));
        let default_name = Path::new(entry)
          .file_name()
          .map(|f| f.to_string_lossy().to_string())
          .unwrap_or_else(|| entry.to_string());
        let (dest_dir, final_name) = prompt_destination(entry, &default_dir, &default_name)?;
        let final_path = finalize_install(&extracted, &dest_dir, &final_name)?;
        chosen_path = Some(dest_dir);
        chosen_name = Some(final_name);
        ct_write_line(style(format!("Copied to {}", final_path.display())).with(Color::Green))?;
      }
    }

    // TODO: implement download and install pipeline using `links[selection]` into `tempdir`.
    Err(AppError::InvalidInput(format!(
      "Native install not yet implemented after selection {:?} in {} (dest: {:?}, name: {:?}, archive_entry: {:?}). Use `termlibs script install ...` for scripts.",
      selected_link.name,
      tempdir.display(),
      chosen_path,
      chosen_name,
      chosen_archive_entry
    )))
  }
}

#[derive(Debug)]
struct NativeInstallPlan {
  target: InstallTarget,
  query: InstallQueryOptions,
}

#[derive(Debug)]
enum InstallTarget {
  SupportedApp(String),
  Github { owner: String, repo: String },
}

impl NativeInstallPlan {
  fn from_args(args: &InstallArgs) -> Result<Self, AppError> {
    let env_os = env::var("TERMLIBS_OS")
      .ok()
      .map(|v| TargetOs::from(v.as_str()));
    let env_arch = env::var("TERMLIBS_ARCH")
      .ok()
      .map(|v| TargetArch::from(v.as_str()));
    let env_version = env::var("TERMLIBS_VERSION").ok();
    let env_prefix = env::var("TERMLIBS_PREFIX").ok();
    let env_method = env::var("TERMLIBS_METHOD")
      .ok()
      .map(|v| InstallMethod::from(v.as_str()));
    let env_download_only = env::var("TERMLIBS_DOWNLOAD_ONLY").ok().map(is_truthy);
    let env_force = env::var("TERMLIBS_FORCE").ok().map(is_truthy);
    let env_quiet = env::var("TERMLIBS_QUIET").ok().map(is_truthy);
    let env_log_level = env::var("TERMLIBS_LOG_LEVEL").ok();

    let os = args
      .os
      .as_ref()
      .map(|v| TargetOs::from(v.as_str()))
      .or(env_os)
      .unwrap_or_else(host_os);
    let arch = args
      .arch
      .as_ref()
      .map(|v| TargetArch::from(v.as_str()))
      .or(env_arch)
      .unwrap_or_else(host_arch);
    let version = args.version.clone().or(env_version);
    let prefix = args.prefix.clone().or(env_prefix);
    let method = args
      .method
      .as_ref()
      .map(|v| InstallMethod::from(v.as_str()))
      .or(env_method);
    let download_only = Some(args.download_only).or(env_download_only);
    let force = Some(args.force).or(env_force);
    let quiet = Some(args.quiet).or(env_quiet);
    let log_level = args.log_level.clone().or(env_log_level);

    let target = match args.target.as_slice() {
      [app] => InstallTarget::SupportedApp(app.to_string()),
      [owner, repo] => InstallTarget::Github {
        owner: owner.to_string(),
        repo: repo.to_string(),
      },
      _ => {
        return Err(AppError::InvalidInput(
          "Expected <app> or <owner> <repo> for install target".to_string(),
        ))
      }
    };

    let query = InstallQueryOptions::new(
      None,
      version,
      prefix,
      Some(arch),
      Some(os),
      method,
      download_only,
      force,
      quiet,
      log_level,
      Some(false),
    );

    Ok(Self { target, query })
  }

  async fn resolve_links(
    &self,
  ) -> Result<(Vec<crate::supported_apps::DownloadInfo>, Vec<String>), AppError> {
    let supported_app = match &self.target {
      InstallTarget::SupportedApp(app) => {
        supported_apps::get_app(app).ok_or_else(|| AppError::UnsupportedApp(app.to_string()))?
      }
      InstallTarget::Github { owner, repo } => {
        let name = format!("{}/{}", owner, repo);
        SupportedApp::new(&name, Repo::github(&name), "github")
      }
    };

    let (_, links) = installer::load_app(&self.query, &supported_app).await?;
    if links.is_empty() {
      return Err(AppError::NoMatchingAssets {
        repo: supported_app.shortname,
        target: self.query.os.to_string(),
      });
    }

    let labels = links
      .iter()
      .map(|link| format!("{} ({})", link.name, link.content_type))
      .collect();

    Ok((links, labels))
  }
}

fn is_truthy(value: String) -> bool {
  matches!(
    value.to_ascii_lowercase().as_str(),
    "1" | "true" | "yes" | "on"
  )
}

fn create_tempdir() -> Result<PathBuf, AppError> {
  let mut path = env::temp_dir();
  let millis = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|d| d.as_millis())
    .unwrap_or(0);
  path.push(format!("termlibs-{}-{}", std::process::id(), millis));
  fs::create_dir_all(&path).map_err(|err| {
    AppError::InvalidInput(format!(
      "failed to create temp dir {}: {}",
      path.display(),
      err
    ))
  })?;
  Ok(path)
}

fn prompt_for_choice(labels: &[String]) -> Result<usize, AppError> {
  if labels.is_empty() {
    return Err(AppError::InvalidInput(
      "no artifacts available to select".to_string(),
    ));
  }

  ct_write_line(style("Please select one of the following:").with(Color::Yellow))?;
  for (idx, label) in labels.iter().enumerate() {
    ct_write_line(style(format!("  {}) {}", idx + 1, label)).with(Color::Cyan))?;
  }
  ct_write(style("Enter choice [default: 1]: ").with(Color::Green))?;

  let mut input = String::new();
  io::stdin()
    .read_line(&mut input)
    .map_err(|err| AppError::InvalidInput(format!("failed to read choice: {}", err)))?;
  let trimmed = input.trim();
  let defaulted = trimmed.is_empty();
  let trimmed = if defaulted { "1" } else { trimmed };
  let choice_one_based: usize = trimmed.parse().map_err(|_| {
    AppError::InvalidInput(format!(
      "invalid choice '{}', expected 1-{}",
      trimmed,
      labels.len()
    ))
  })?;
  if choice_one_based == 0 || choice_one_based > labels.len() {
    return Err(AppError::InvalidInput(format!(
      "choice out of range: {}",
      choice_one_based
    )));
  }
  Ok(choice_one_based - 1)
}

fn prompt_destination(
  label: &str,
  default_dir: &Path,
  default_name: &str,
) -> Result<(PathBuf, String), AppError> {
  ct_write_line(style(format!("Selected file: {}", label)).with(Color::Yellow))?;
  ct_write(
    style(format!(
      "Enter install directory [default: {}]: ",
      default_dir.display()
    ))
    .with(Color::Green),
  )?;

  let mut dir_input = String::new();
  io::stdin()
    .read_line(&mut dir_input)
    .map_err(|err| AppError::InvalidInput(format!("failed to read directory: {}", err)))?;
  let dir_input = dir_input.trim();
  let dest_dir = if dir_input.is_empty() {
    default_dir.to_path_buf()
  } else {
    PathBuf::from(dir_input)
  };

  ct_write(style(format!("Enter file name [default: {}]: ", default_name)).with(Color::Green))?;

  let mut name_input = String::new();
  io::stdin()
    .read_line(&mut name_input)
    .map_err(|err| AppError::InvalidInput(format!("failed to read name: {}", err)))?;
  let name = name_input.trim();
  let final_name = if name.is_empty() {
    default_name.to_string()
  } else {
    name.to_string()
  };

  Ok((dest_dir, final_name))
}

fn is_probably_binary(content_type: &Mime) -> bool {
  let top = content_type.type_().as_str();
  let sub = content_type.subtype().as_str();
  matches!(
    (top, sub),
    ("application", "octet-stream")
      | ("application", "x-msdownload")
      | ("application", "x-executable")
      | ("application", "x-binary")
  )
}

fn default_binary_name(
  target: &InstallTarget,
  link: &crate::supported_apps::DownloadInfo,
) -> String {
  let ext = Path::new(&link.name)
    .extension()
    .map(|e| format!(".{}", e.to_string_lossy()))
    .unwrap_or_default();
  match target {
    InstallTarget::SupportedApp(name) => format!("{}{}", name, ext),
    InstallTarget::Github { .. } => link.name.clone(),
  }
}

fn is_archive(content_type: &Mime, name: &str) -> bool {
  let sub = content_type.subtype().as_str();
  let full = content_type.to_string();
  let lower_name = name.to_ascii_lowercase();
  full.contains("zip")
    || sub.contains("zip")
    || sub.contains("tar")
    || lower_name.ends_with(".zip")
    || lower_name.ends_with(".tar")
    || lower_name.ends_with(".tar.gz")
    || lower_name.ends_with(".tgz")
    || lower_name.ends_with(".tar.xz")
}

async fn download_asset(
  url: &url::Url,
  tempdir: &Path,
  filename: &str,
) -> Result<PathBuf, AppError> {
  let resp = reqwest::get(url.as_str())
    .await
    .map_err(|err| AppError::InvalidInput(format!("failed to download {}: {}", url, err)))?;
  if !resp.status().is_success() {
    return Err(AppError::InvalidInput(format!(
      "download failed for {}: status {}",
      url,
      resp.status()
    )));
  }
  let bytes = resp
    .bytes()
    .await
    .map_err(|err| AppError::InvalidInput(format!("failed to read body {}: {}", url, err)))?;
  let mut path = tempdir.to_path_buf();
  path.push(filename);
  fs::write(&path, &bytes).map_err(|err| {
    AppError::InvalidInput(format!("failed to write {}: {}", path.display(), err))
  })?;
  Ok(path)
}

fn list_archive_entries(path: &Path) -> Result<Vec<String>, AppError> {
  let lower = path
    .extension()
    .map(|e| e.to_string_lossy().to_ascii_lowercase())
    .unwrap_or_default();
  if lower == "zip" {
    let file = File::open(path).map_err(|err| {
      AppError::InvalidInput(format!("failed to open {}: {}", path.display(), err))
    })?;
    let mut archive = ZipArchive::new(file).map_err(|err| {
      AppError::InvalidInput(format!("failed to read zip {}: {}", path.display(), err))
    })?;
    let mut entries = Vec::new();
    for i in 0..archive.len() {
      let file = archive
        .by_index(i)
        .map_err(|err| AppError::InvalidInput(format!("failed to read zip entry: {}", err)))?;
      let name = file.name().to_string();
      entries.push(name);
    }
    return Ok(limit_depth(entries, 2));
  }

  if lower == "gz"
    || path.to_string_lossy().ends_with(".tar.gz")
    || path.to_string_lossy().ends_with(".tgz")
  {
    let file = File::open(path).map_err(|err| {
      AppError::InvalidInput(format!("failed to open {}: {}", path.display(), err))
    })?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    let mut entries = Vec::new();
    for entry in archive
      .entries()
      .map_err(|err| AppError::InvalidInput(format!("failed to read tar: {}", err)))?
    {
      let entry = entry
        .map_err(|err| AppError::InvalidInput(format!("failed to read tar entry: {}", err)))?;
      let path = entry
        .path()
        .map_err(|err| AppError::InvalidInput(format!("failed to read tar path: {}", err)))?;
      let path_str = path.to_string_lossy().to_string();
      entries.push(path_str);
    }
    return Ok(limit_depth(entries, 2));
  }

  if lower == "tar" {
    let file = File::open(path).map_err(|err| {
      AppError::InvalidInput(format!("failed to open {}: {}", path.display(), err))
    })?;
    let mut archive = Archive::new(file);
    let mut entries = Vec::new();
    for entry in archive
      .entries()
      .map_err(|err| AppError::InvalidInput(format!("failed to read tar: {}", err)))?
    {
      let entry = entry
        .map_err(|err| AppError::InvalidInput(format!("failed to read tar entry: {}", err)))?;
      let path = entry
        .path()
        .map_err(|err| AppError::InvalidInput(format!("failed to read tar path: {}", err)))?;
      let path_str = path.to_string_lossy().to_string();
      entries.push(path_str);
    }
    return Ok(limit_depth(entries, 2));
  }

  Err(AppError::InvalidInput(
    "unsupported archive format".to_string(),
  ))
}

fn limit_depth(entries: Vec<String>, depth: usize) -> Vec<String> {
  entries
    .into_iter()
    .map(|entry| {
      let parts = entry.split('/').take(depth).collect::<Vec<_>>();
      let mut joined = parts.join("/");
      if entry.ends_with('/') && !joined.ends_with('/') {
        joined.push('/');
      }
      joined
    })
    .collect()
}

fn render_tree(entries: &[String]) -> Result<(), AppError> {
  ct_write_line(style("Archive contents (depth 2):").with(Color::Yellow))?;
  for entry in entries {
    let indent = entry.matches('/').count();
    let prefix = "  ".repeat(indent.min(2));
    ct_write_line(style(format!("{}- {}", prefix, entry)).with(Color::Cyan))?;
  }
  Ok(())
}

fn extract_archive_entry(
  archive_path: &Path,
  entry_name: &str,
  tempdir: &Path,
) -> Result<PathBuf, AppError> {
  let lower = archive_path
    .extension()
    .map(|e| e.to_string_lossy().to_ascii_lowercase())
    .unwrap_or_default();

  if lower == "zip" {
    let file = File::open(archive_path).map_err(|err| {
      AppError::InvalidInput(format!(
        "failed to open {}: {}",
        archive_path.display(),
        err
      ))
    })?;
    let mut archive = ZipArchive::new(file).map_err(|err| {
      AppError::InvalidInput(format!(
        "failed to read zip {}: {}",
        archive_path.display(),
        err
      ))
    })?;
    let mut zip_file = archive.by_name(entry_name).map_err(|err| {
      AppError::InvalidInput(format!("failed to open zip entry {}: {}", entry_name, err))
    })?;
    let file_name = Path::new(entry_name)
      .file_name()
      .map(|f| f.to_string_lossy().to_string())
      .unwrap_or_else(|| entry_name.to_string());
    let mut out_path = tempdir.to_path_buf();
    out_path.push(file_name);
    let mut out = File::create(&out_path).map_err(|err| {
      AppError::InvalidInput(format!("failed to create {}: {}", out_path.display(), err))
    })?;
    io::copy(&mut zip_file, &mut out).map_err(|err| {
      AppError::InvalidInput(format!("failed to extract {}: {}", entry_name, err))
    })?;
    return Ok(out_path);
  }

  let is_gzip = lower == "gz"
    || archive_path.to_string_lossy().ends_with(".tar.gz")
    || archive_path.to_string_lossy().ends_with(".tgz");
  let is_tar = lower == "tar" || is_gzip;
  if is_tar {
    let file = File::open(archive_path).map_err(|err| {
      AppError::InvalidInput(format!(
        "failed to open {}: {}",
        archive_path.display(),
        err
      ))
    })?;
    if is_gzip {
      let decoder = GzDecoder::new(file);
      let mut archive = Archive::new(decoder);
      for entry in archive
        .entries()
        .map_err(|err| AppError::InvalidInput(format!("failed to read tar: {}", err)))?
      {
        let mut entry = entry
          .map_err(|err| AppError::InvalidInput(format!("failed to read tar entry: {}", err)))?;
        let path = entry
          .path()
          .map_err(|err| AppError::InvalidInput(format!("failed to read tar path: {}", err)))?;
        let path_str = path.to_string_lossy().to_string();
        if path_str == entry_name {
          let file_name = Path::new(entry_name)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| entry_name.to_string());
          let mut out_path = tempdir.to_path_buf();
          out_path.push(file_name);
          let mut out = File::create(&out_path).map_err(|err| {
            AppError::InvalidInput(format!("failed to create {}: {}", out_path.display(), err))
          })?;
          io::copy(&mut entry, &mut out).map_err(|err| {
            AppError::InvalidInput(format!("failed to extract {}: {}", entry_name, err))
          })?;
          return Ok(out_path);
        }
      }
    } else {
      let mut archive = Archive::new(file);
      for entry in archive
        .entries()
        .map_err(|err| AppError::InvalidInput(format!("failed to read tar: {}", err)))?
      {
        let mut entry = entry
          .map_err(|err| AppError::InvalidInput(format!("failed to read tar entry: {}", err)))?;
        let path = entry
          .path()
          .map_err(|err| AppError::InvalidInput(format!("failed to read tar path: {}", err)))?;
        let path_str = path.to_string_lossy().to_string();
        if path_str == entry_name {
          let file_name = Path::new(entry_name)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| entry_name.to_string());
          let mut out_path = tempdir.to_path_buf();
          out_path.push(file_name);
          let mut out = File::create(&out_path).map_err(|err| {
            AppError::InvalidInput(format!("failed to create {}: {}", out_path.display(), err))
          })?;
          io::copy(&mut entry, &mut out).map_err(|err| {
            AppError::InvalidInput(format!("failed to extract {}: {}", entry_name, err))
          })?;
          return Ok(out_path);
        }
      }
    }
    return Err(AppError::InvalidInput(format!(
      "entry not found in archive: {}",
      entry_name
    )));
  }

  Err(AppError::InvalidInput(
    "unsupported archive format".to_string(),
  ))
}

fn finalize_install(source: &Path, dest_dir: &Path, dest_name: &str) -> Result<PathBuf, AppError> {
  fs::create_dir_all(dest_dir).map_err(|err| {
    AppError::InvalidInput(format!("failed to create {}: {}", dest_dir.display(), err))
  })?;
  let mut dest = dest_dir.to_path_buf();
  dest.push(dest_name);
  fs::copy(source, &dest).map_err(|err| {
    AppError::InvalidInput(format!("failed to copy to {}: {}", dest.display(), err))
  })?;
  Ok(dest)
}

fn ct_write_line(message: impl std::fmt::Display) -> Result<(), AppError> {
  execute!(io::stdout(), Print(message), Print("\n"))
    .map_err(|err| AppError::InvalidInput(format!("failed to write: {}", err)))
}

fn ct_write(message: impl std::fmt::Display) -> Result<(), AppError> {
  execute!(io::stdout(), Print(message))
    .map_err(|err| AppError::InvalidInput(format!("failed to write: {}", err)))
}

#[derive(Subcommand)]
pub(crate) enum ScriptCommands {
  /// Generate an install script (mirrors /v1/install API)
  Install(ScriptInstallArgs),
}

#[derive(Args, Debug)]
pub(crate) struct ScriptInstallArgs {
  /// Target to install: <app> or <owner> <repo>
  #[arg(value_name = "APP|OWNER REPO", num_args = 1..=2)]
  target: Vec<String>,
  /// Target operating system
  #[arg(long)]
  os: Option<String>,
  /// Target architecture
  #[arg(long)]
  arch: Option<String>,
  /// Release version or tag (default: latest)
  #[arg(long)]
  version: Option<String>,
  /// Installation prefix (default: $HOME/.local)
  #[arg(long)]
  prefix: Option<String>,
  /// Install method hint: binary or installer
  #[arg(long)]
  method: Option<String>,
  /// Download-only mode
  #[arg(long)]
  download_only: bool,
  /// Force installation
  #[arg(long)]
  force: bool,
  /// Quiet mode
  #[arg(long)]
  quiet: bool,
  /// Log level injected into the script
  #[arg(long)]
  log_level: Option<String>,
  /// Output JSON map of filename to download URL (no script rendering)
  #[arg(long)]
  links_only: bool,
}

impl ScriptInstallArgs {
  pub(crate) async fn run(&self) -> Result<CliInstallOutput, AppError> {
    let os = self
      .os
      .as_ref()
      .map(|value| TargetOs::from(value.as_str()))
      .unwrap_or_else(host_os);
    let arch = self
      .arch
      .as_ref()
      .map(|value| TargetArch::from(value.as_str()))
      .unwrap_or_else(host_arch);
    let method = self
      .method
      .as_ref()
      .map(|value| InstallMethod::from(value.as_str()));
    let mut query = InstallQueryOptions::new(
      None,
      self.version.clone(),
      self.prefix.clone(),
      Some(arch),
      Some(os),
      method,
      Some(self.download_only),
      Some(self.force),
      Some(self.quiet),
      self.log_level.clone(),
      Some(false),
    );

    if self.links_only {
      let links = match self.target.as_slice() {
        [app] => {
          let supported_app = supported_apps::get_app(app)
            .ok_or_else(|| AppError::UnsupportedApp(app.to_string()))?;
          let (_, links) = installer::load_app(&query, &supported_app).await?;
          links
        }
        [user, repo] => {
          let app_name = format!("{}/{}", user, repo);
          let supported_app = SupportedApp::new(&app_name, Repo::github(&app_name), "github");
          let (_, links) = installer::load_app(&query, &supported_app).await?;
          links
        }
        _ => {
          return Err(AppError::InvalidInput(
            "Expected <app> or <owner> <repo> for install target".to_string(),
          ));
        }
      };
      Ok(CliInstallOutput::Links(render_links_for_cli(&links)))
    } else {
      let response = match self.target.as_slice() {
        [app] => installer::build_supported_install_script(app, &mut query, false).await,
        [user, repo] => {
          installer::build_arbitrary_github_install_script(user, repo, &mut query, false).await
        }
        _ => {
          return Err(AppError::InvalidInput(
            "Expected <app> or <owner> <repo> for install target".to_string(),
          ));
        }
      }?;
      Ok(CliInstallOutput::Script(response))
    }
  }
}

pub(crate) enum CliInstallOutput {
  Script(ScriptResponse),
  Links(String),
}

fn host_os() -> TargetOs {
  let os = env::consts::OS;
  TargetOs::identify(os)
}

fn host_arch() -> TargetArch {
  let arch = env::consts::ARCH;
  TargetArch::identify(arch)
}

fn render_links_for_cli(links: &[crate::supported_apps::DownloadInfo]) -> String {
  let mut map = serde_json::Map::with_capacity(links.len());
  for link in links {
    map.insert(link.name.clone(), Value::String(link.url.to_string()));
  }
  serde_json::to_string(&Value::Object(map)).unwrap_or_else(|_| "{}".to_string())
}

pub(crate) fn parse() -> Cli {
  Cli::parse()
}

pub(crate) fn build_command() -> clap::Command {
  Cli::command()
}
