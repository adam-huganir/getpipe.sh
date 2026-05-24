use crate::domain::platform::TargetDeployment;
use crate::error::AppError;
use crate::http::query::InstallQueryOptions;
use crate::http::responses::ScriptResponse;
use crate::providers::gh::get_github_download_links;
use crate::services::templating;
use crate::supported_apps;
use crate::supported_apps::{DownloadInfo, Repo, SupportedApp};
use log::debug;

fn validate_github_path_segment(segment: &str, name: &str) -> Result<(), AppError> {
  if segment.is_empty() {
    return Err(AppError::InvalidInput(format!("{} cannot be empty", name)));
  }

  if segment.len() > 100 {
    return Err(AppError::InvalidInput(format!(
      "{} exceeds maximum length of 100 characters",
      name
    )));
  }

  // Check for path traversal attempts
  if segment.contains("..") || segment.contains('/') || segment.contains('\\') {
    return Err(AppError::InvalidInput(format!(
      "{} contains invalid characters",
      name
    )));
  }

  // GitHub usernames and repos can only contain alphanumeric, hyphens, underscores, and dots
  // but dots cannot be used for path traversal
  if !segment
    .chars()
    .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
  {
    return Err(AppError::InvalidInput(format!(
      "{} contains invalid characters",
      name
    )));
  }

  // Additional safety: reject segments that start with a dot
  if segment.starts_with('.') {
    return Err(AppError::InvalidInput(format!(
      "{} cannot start with a dot",
      name
    )));
  }

  Ok(())
}

pub(crate) async fn build_supported_install_script(
  app: &str,
  query: &mut InstallQueryOptions,
  html: bool,
) -> Result<ScriptResponse, AppError> {
  query.set_app(app.to_string());

  let supported_app =
    supported_apps::get_app(app).ok_or_else(|| AppError::UnsupportedApp(app.to_string()))?;

  let (target, links) = load_app(query, &supported_app).await?;
  let (script, extension) = templating::render_install_script(query, &links, &target.os)?;

  Ok(ScriptResponse::new(
    format!("install-{}.{}", supported_app.shortname, extension),
    script,
    query.inline,
    html,
  ))
}

pub(crate) async fn build_arbitrary_github_install_script(
  user: &str,
  repo: &str,
  query: &mut InstallQueryOptions,
  html: bool,
) -> Result<ScriptResponse, AppError> {
  validate_github_path_segment(user, "user")?;
  validate_github_path_segment(repo, "repo")?;

  let app_name = format!("{}/{}", user, repo);
  let target_app = SupportedApp::new(&app_name, Repo::github(&app_name), "github");

  query.set_app(app_name);
  let (target, links) = load_app(query, &target_app).await?;
  let (script, extension) = templating::render_install_script(query, &links, &target.os)?;

  Ok(ScriptResponse::new(
    format!("install.{}", extension),
    script,
    query.inline,
    html,
  ))
}

pub(crate) async fn load_app(
  query: &InstallQueryOptions,
  supported_app: &SupportedApp,
) -> Result<(TargetDeployment, Vec<DownloadInfo>), AppError> {
  let arch = query.arch.clone();
  let os = query.os.clone();
  let version = query.version.clone();
  let target_deployment = TargetDeployment::new(os, arch);
  debug!("target_deployment loaded: {:#?}", target_deployment);

  let links = get_github_download_links(&supported_app.repo, &target_deployment, &version).await?;
  if links.is_empty() {
    return Err(AppError::NoMatchingAssets {
      repo: supported_app.shortname.clone(),
      target: target_deployment.to_string(),
    });
  }

  Ok((target_deployment, links))
}
