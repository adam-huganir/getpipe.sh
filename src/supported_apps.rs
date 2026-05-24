use crate::domain::download::Target;
use crate::domain::platform::TargetDeployment;
use crate::error::AppError;
use crate::providers::gh::get_github_download_links;
use mime::Mime;
use serde_json::json;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::LazyLock;
use url::Url;

macro_rules! safe_int_cast {
  ($val:expr, $target_type:ty) => {{
    if $val < 0 {
      0 as $target_type
    } else {
      $val as $target_type
    }
  }};
}

const GITHUB_API: &str = "https://api.github.com";

pub(crate) fn get_app(name: &str) -> Option<SupportedApp> {
  SUPPORTED_APPS.get(name).cloned()
}

#[derive(Debug, Clone)]
pub(crate) struct SupportedApp {
  pub(crate) shortname: String,
  pub(crate) repo: Repo,
  #[allow(dead_code)]
  pub(crate) source: String,
}

impl SupportedApp {
  pub(crate) fn new(shortname: &str, repo: Repo, source: &str) -> Self {
    Self {
      shortname: shortname.to_string(),
      repo,
      source: source.to_string(),
    }
  }
}

static SUPPORTED_APPS: LazyLock<HashMap<&str, SupportedApp>> = LazyLock::new(|| {
  let mut map = HashMap::new();
  for (app, github_url) in [
    ("yq", "mikefarah/yq"),
    ("jq", "jqlang/jq"),
    ("gh", "cli/cli"),
    ("jsonnet", "google/go-jsonnet"),
    ("shellcheck", "koalaman/shellcheck"),
    ("shfmt", "mvdan/sh"),
    ("yutc", "adam-huganir/yutc"),
    ("kubectl", "kubernetes/kubectl"),
    ("helm", "helm/helm"),
    ("uv", "astral-sh/uv"),
  ] {
    let _ = map.insert(
      app,
      SupportedApp::new(app, Repo::github(github_url), "github"),
    );
  }
  map
});

#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub(crate) enum Repo {
  Github(String),
  #[allow(dead_code)]
  Url(String),
  #[allow(dead_code)]
  Python(String),
}

impl Repo {
  pub(crate) fn github(repo: &str) -> Self {
    Self::Github(format!("{}/repos/{}", GITHUB_API, repo))
  }

  #[allow(dead_code)]
  fn url(url: &str) -> Self {
    Self::Url(url.to_string())
  }

  #[allow(dead_code)]
  fn python(app: &str) -> Self {
    Self::Python(format!("https://pypi.org/simple/{}", app))
  }

  fn get_url(&self) -> Result<Url, AppError> {
    let parsed = match self {
      Repo::Github(repo) => Url::parse(repo),
      Repo::Url(url) => Url::parse(url),
      Repo::Python(url) => Url::parse(url),
    };
    parsed.map_err(|err| AppError::InvalidInput(format!("Invalid repo URL: {}", err)))
  }

  pub(crate) fn get_github_repo(&self) -> Result<String, AppError> {
    Ok(
      self
        .get_url()?
        .path()
        .trim_start_matches("/repos/")
        .to_string(),
    )
  }

  #[allow(dead_code)]
  pub(crate) async fn get_download_link(
    &self,
    version: &str,
    target_deployment: &TargetDeployment,
  ) -> Result<Vec<DownloadInfo>, AppError> {
    match self {
      Repo::Github(_) => get_github_download_links(self, target_deployment, version).await,
      Repo::Url(url) => Err(AppError::InvalidInput(format!(
        "{} is not a github repo",
        url
      ))),
      Repo::Python(url) => Err(AppError::InvalidInput(format!(
        "{} is not a github repo",
        url
      ))),
    }
  }
}

#[derive(Debug)]
pub(crate) struct DownloadInfo {
  pub(crate) name: String,
  pub(crate) label: String,
  pub(crate) url: Url,
  pub(crate) content_type: Mime,
  pub(crate) size: u64,
  pub(crate) target: Target,
}

impl DownloadInfo {
  pub(crate) fn from_asset(asset: &octocrab::models::repos::Asset) -> Self {
    let mime = asset
      .content_type
      .parse::<Mime>()
      .unwrap_or(mime::APPLICATION_OCTET_STREAM);

    let size = safe_int_cast!(asset.size, u64);

    Self {
      name: asset.name.clone(),
      label: asset.label.to_owned().unwrap_or("".to_string()),
      url: asset.browser_download_url.clone(),
      content_type: mime.to_owned(),
      size,
      target: Target::identify(&asset.name, Some(&mime)),
    }
  }

  pub(crate) fn json(&self) -> serde_json::Value {
    json!({
        "name": self.name,
        "label": self.label,
        "url": self.url.to_string(),
        "content_type": self.content_type.to_string(),
        "filetype": self.target.filetype.to_string(),
        "os": self.target.deployment.os.to_string(),
        "arch": self.target.deployment.arch.to_string(),
        "size": self.size
    })
  }
}

impl Display for DownloadInfo {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "[{}]({}) for {} as a {}",
      self.name, self.url, self.target.deployment, self.target.filetype
    )
  }
}
