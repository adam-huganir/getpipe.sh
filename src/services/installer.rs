use crate::domain::platform::TargetDeployment;
use crate::error::AppError;
use crate::http::query::InstallQueryOptions;
use crate::http::responses::ScriptResponse;
use crate::providers::gh::{get_github_download_links, get_github_release_tags};
use crate::services::templating;
use crate::supported_apps;
use crate::supported_apps::{DownloadInfo, Repo, SupportedApp};
use log::debug;

pub(crate) fn validate_github_path_segment(segment: &str, name: &str) -> Result<(), AppError> {
  if segment.is_empty() {
    return Err(AppError::InvalidInput(format!("{} cannot be empty", name)));
  }

  if segment.len() > 100 {
    return Err(AppError::InvalidInput(format!(
      "{} exceeds maximum length of 100 characters",
      name
    )));
  }

  if !segment
    .chars()
    .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
  {
    return Err(AppError::InvalidInput(format!(
      "{} contains invalid characters",
      name
    )));
  }

  if segment.starts_with('.') {
    return Err(AppError::InvalidInput(format!(
      "{} cannot start with a dot",
      name
    )));
  }

  Ok(())
}

async fn fetch_tags_with_latest(repo: &Repo) -> Vec<String> {
  let mut tags = get_github_release_tags(repo, 20).await.unwrap_or_default();
  tags.insert(0, "latest".to_string());
  tags
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

  let mut response = ScriptResponse::new(
    format!("install-{}.{}", supported_app.shortname, extension),
    script,
    query.inline,
    html,
  );

  if html {
    response = response.with_tags(fetch_tags_with_latest(&supported_app.repo).await);
  }

  Ok(response)
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
  let target_app = SupportedApp::new(&app_name, Repo::github(&app_name));

  query.set_app(app_name);
  let (target, links) = load_app(query, &target_app).await?;
  let (script, extension) = templating::render_install_script(query, &links, &target.os)?;

  let mut response =
    ScriptResponse::new(format!("install.{}", extension), script, query.inline, html);

  if html {
    response = response.with_tags(fetch_tags_with_latest(&target_app.repo).await);
  }

  Ok(response)
}

pub(crate) async fn load_app(
  query: &InstallQueryOptions,
  supported_app: &SupportedApp,
) -> Result<(TargetDeployment, Vec<DownloadInfo>), AppError> {
  let target_deployment = TargetDeployment::new(query.os.clone(), query.arch.clone());
  debug!("target_deployment loaded: {:#?}", target_deployment);

  let links =
    get_github_download_links(&supported_app.repo, &target_deployment, &query.version).await?;
  if links.is_empty() {
    return Err(AppError::NoMatchingAssets {
      repo: supported_app.shortname.clone(),
      target: target_deployment.to_string(),
    });
  }

  Ok((target_deployment, links))
}
