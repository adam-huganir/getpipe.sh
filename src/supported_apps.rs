use crate::domain::download::Target;
use crate::error::AppError;
use mime::Mime;
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

pub(crate) fn get_app(name: &str) -> Option<SupportedApp> {
  SUPPORTED_APPS.get(name).cloned()
}

/// Returns all supported apps sorted alphabetically by shortname.
pub(crate) fn list_apps() -> Vec<(&'static str, &'static SupportedApp)> {
  let mut apps: Vec<_> = SUPPORTED_APPS.iter().map(|(k, v)| (*k, v)).collect();
  apps.sort_by_key(|(name, _)| *name);
  apps
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
    // --- Query / data tools ---
    ("yq", "mikefarah/yq"),
    ("jq", "jqlang/jq"),
    ("jsonnet", "google/go-jsonnet"),
    // --- GitHub CLI ---
    ("gh", "cli/cli"),
    // --- Shell / script utilities ---
    ("shellcheck", "koalaman/shellcheck"),
    ("shfmt", "mvdan/sh"),
    // --- Terminal utilities ---
    ("rg", "BurntSushi/ripgrep"),
    ("fd", "sharkdp/fd"),
    ("bat", "sharkdp/bat"),
    ("delta", "dandavison/delta"),
    ("fzf", "junegunn/fzf"),
    ("zoxide", "ajeetdsouza/zoxide"),
    ("starship", "starship-rs/starship"),
    ("lazygit", "jesseduffield/lazygit"),
    ("eza", "eza-community/eza"),
    // --- Build / task runners ---
    ("just", "casey/just"),
    ("task", "go-task/task"),
    ("mise", "jdx/mise"),
    ("goreleaser", "goreleaser/goreleaser"),
    // --- Kubernetes / infrastructure ---
    ("kubectl", "kubernetes/kubectl"),
    ("helm", "helm/helm"),
    ("k9s", "derailed/k9s"),
    ("flux", "fluxcd/flux2"),
    ("cilium", "cilium/cilium-cli"),
    ("kustomize", "kubernetes-sigs/kustomize"),
    ("istioctl", "istio/istio"),
    ("stern", "stern/stern"),
    // --- Disk utilities ---
    ("dust", "bootandy/dust"),
    // --- Security / secrets ---
    ("age", "FiloSottile/age"),
    ("sops", "getsops/sops"),
    ("cosign", "sigstore/cosign"),
    ("syft", "anchore/syft"),
    // --- Python tooling ---
    ("uv", "astral-sh/uv"),
    // --- Package managers ---
    ("bin", "marcosnils/bin"),
    // --- Misc ---
    ("yutc", "adam-huganir/yutc"),
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
  /// Placeholder for future support of arbitrary URL download sources
  /// (e.g. direct asset hosting outside GitHub Releases).
  /// Not yet implemented gcr.io/getpipe-sh kept here to reserve the variant and signal intent.
  #[allow(dead_code)]
  Url(String),
  /// Placeholder for future support of PyPI packages as an install source.
  /// Not yet implemented — kept here to reserve the variant and signal intent.
  #[allow(dead_code)]
  Python(String),
}

impl Repo {
  pub(crate) fn github(repo: &str) -> Self {
    Self::Github(format!("https://api.github.com/repos/{}", repo))
  }

  /// Constructs a repo backed by a direct download URL.
  /// Not yet wired into the installer — see the `Url` variant doc comment.
  #[allow(dead_code)]
  pub(crate) fn url(url: &str) -> Self {
    Self::Url(url.to_string())
  }

  /// Constructs a repo backed by a PyPI package name.
  /// Not yet wired into the installer — see the `Python` variant doc comment.
  #[allow(dead_code)]
  pub(crate) fn python(package: &str) -> Self {
    Self::Python(package.to_string())
  }

  fn get_url(&self) -> Result<Url, AppError> {
    match self {
      Repo::Github(repo) => {
        Url::parse(repo).map_err(|err| AppError::InvalidInput(format!("Invalid repo URL: {}", err)))
      }
      Repo::Url(url) => {
        Url::parse(url).map_err(|err| AppError::InvalidInput(format!("Invalid URL: {}", err)))
      }
      Repo::Python(_) => Err(AppError::InvalidInput(
        "PyPI source is not yet implemented".to_string(),
      )),
    }
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

  /// Serializes this asset for use as a Tera template variable.
  /// Delegates to the `Serialize` impl; adding a field there automatically
  /// makes it available in templates.
  pub(crate) fn json(&self) -> serde_json::Value {
    // Serialization of in-memory string/integer fields is infallible.
    serde_json::to_value(self).expect("DownloadInfo serialization is infallible")
  }
}

/// Custom `Serialize` impl for `DownloadInfo`.
///
/// `Mime` and the domain enums (`Filetype`, `TargetOs`, `TargetArch`) do not
/// serialize to the string representation that templates expect, so each field
/// is explicitly converted to its `Display` form.  When you add a new field to
/// `DownloadInfo` that templates need, add a matching line here — the compiler
/// will remind you if you forget to update the field count in
/// `serialize_struct`.
impl serde::Serialize for DownloadInfo {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    use serde::ser::SerializeStruct;
    let mut s = serializer.serialize_struct("DownloadInfo", 8)?;
    s.serialize_field("name", &self.name)?;
    s.serialize_field("label", &self.label)?;
    s.serialize_field("url", &self.url.to_string())?;
    s.serialize_field("content_type", &self.content_type.to_string())?;
    s.serialize_field("filetype", &self.target.filetype.to_string())?;
    s.serialize_field("os", &self.target.deployment.os.to_string())?;
    s.serialize_field("arch", &self.target.deployment.arch.to_string())?;
    s.serialize_field("size", &self.size)?;
    s.end()
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
