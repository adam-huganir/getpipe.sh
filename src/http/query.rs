use crate::domain::platform::{TargetArch, TargetOs};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::fmt::Display;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize, ToSchema)]
pub(crate) enum InstallMethod {
  Installer,
  Binary,
}

impl Display for InstallMethod {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      InstallMethod::Installer => write!(f, "installer"),
      InstallMethod::Binary => write!(f, "binary"),
    }
  }
}

impl From<&str> for InstallMethod {
  fn from(value: &str) -> Self {
    match value {
      "installer" => InstallMethod::Installer,
      _ => InstallMethod::Binary,
    }
  }
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize, ToSchema, IntoParams)]
pub(crate) struct InstallQueryOptions {
  #[serde(skip)]
  app: Option<String>,
  #[serde(default = "default_latest")]
  pub(crate) version: String,
  #[serde(default = "default_prefix")]
  prefix: String,
  #[serde(default = "default_arch")]
  pub(crate) arch: TargetArch,
  #[serde(default = "default_os")]
  pub(crate) os: TargetOs,
  #[serde(default = "default_method")]
  method: InstallMethod,
  #[serde(default = "default_download_only")]
  download_only: bool,
  #[serde(default = "default_force")]
  force: bool,
  #[serde(default = "default_quiet")]
  quiet: bool,
  #[serde(default = "default_log_level")]
  pub(crate) log_level: String,
  #[serde(default = "default_inline")]
  pub(crate) inline: bool,
}

fn default_latest() -> String {
  "latest".to_string()
}

fn default_prefix() -> String {
  "$HOME/.local".to_string()
}

fn default_arch() -> TargetArch {
  TargetArch::Amd64
}

fn default_os() -> TargetOs {
  TargetOs::Linux
}

fn default_method() -> InstallMethod {
  InstallMethod::Binary
}

fn default_download_only() -> bool {
  false
}

fn default_force() -> bool {
  false
}

fn default_quiet() -> bool {
  false
}

fn default_log_level() -> String {
  "DEBUG".to_string()
}

fn default_inline() -> bool {
  false
}

impl InstallQueryOptions {
  #[allow(clippy::too_many_arguments)]
  pub(crate) fn new(
    app: Option<String>,
    version: Option<String>,
    prefix: Option<String>,
    arch: Option<TargetArch>,
    os: Option<TargetOs>,
    method: Option<InstallMethod>,
    download_only: Option<bool>,
    force: Option<bool>,
    quiet: Option<bool>,
    log_level: Option<String>,
    inline: Option<bool>,
  ) -> Self {
    Self {
      app,
      version: version.unwrap_or_else(default_latest),
      prefix: prefix.unwrap_or_else(default_prefix),
      arch: arch.unwrap_or_else(default_arch),
      os: os.unwrap_or_else(default_os),
      method: method.unwrap_or_else(default_method),
      download_only: download_only.unwrap_or_else(default_download_only),
      force: force.unwrap_or_else(default_force),
      quiet: quiet.unwrap_or_else(default_quiet),
      log_level: log_level.unwrap_or_else(default_log_level),
      inline: inline.unwrap_or_else(default_inline),
    }
  }

  pub(crate) fn set_app(&mut self, app: String) {
    self.app = Some(app);
  }

  pub(crate) fn template_globals(&self) -> Map<String, Value> {
    json!({
        "app": self.app.as_deref().unwrap_or(""),
        "version": self.version.as_str(),
        "prefix": self.prefix.as_str(),
        "arch": self.arch.to_string(),
        "os": self.os.to_string(),
        "method": self.method.to_string(),
        "download_only": self.download_only,
        "force": self.force,
        "quiet": self.quiet,
        "log_level": self.log_level.as_str(),
        "inline": self.inline,
    })
    .as_object()
    .unwrap()
    .to_owned()
  }
}
