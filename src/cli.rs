use clap::{Args, CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

use crate::domain::platform::{TargetArch, TargetOs};
use crate::error::AppError;
use crate::http::query::{InstallMethod, InstallQueryOptions};
use crate::http::responses::ScriptResponse;
use crate::services::installer;
use crate::supported_apps::{self, Repo, SupportedApp};
use serde_json::Value;
use std::env;

#[derive(Parser)]
#[command(name = "getpipe")]
#[command(about = "getpipe.sh server and install script CLI")]
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
      .map(|v| TargetOs::from(v.as_str()))
      .unwrap_or_else(host_os);
    let arch = self
      .arch
      .as_ref()
      .map(|v| TargetArch::from(v.as_str()))
      .unwrap_or_else(host_arch);
    let method = self.method.as_ref().map(|v| InstallMethod::from(v.as_str()));
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
      let supported_app = match self.target.as_slice() {
        [app] => supported_apps::get_app(app)
          .ok_or_else(|| AppError::UnsupportedApp(app.to_string()))?,
        [user, repo] => {
          let name = format!("{}/{}", user, repo);
          SupportedApp::new(&name, Repo::github(&name))
        }
        _ => {
          return Err(AppError::InvalidInput(
            "Expected <app> or <owner> <repo> for install target".to_string(),
          ))
        }
      };
      let (_, links) = installer::load_app(&query, &supported_app).await?;
      return Ok(CliInstallOutput::Links(render_links_for_cli(&links)));
    }

    let response = match self.target.as_slice() {
      [app] => installer::build_supported_install_script(app, &mut query, false).await,
      [user, repo] => {
        installer::build_arbitrary_github_install_script(user, repo, &mut query, false).await
      }
      _ => {
        return Err(AppError::InvalidInput(
          "Expected <app> or <owner> <repo> for install target".to_string(),
        ))
      }
    }?;
    Ok(CliInstallOutput::Script(response))
  }
}

pub(crate) enum CliInstallOutput {
  Script(ScriptResponse),
  Links(String),
}

fn host_os() -> TargetOs {
  TargetOs::identify(env::consts::OS)
}

fn host_arch() -> TargetArch {
  TargetArch::identify(env::consts::ARCH)
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
