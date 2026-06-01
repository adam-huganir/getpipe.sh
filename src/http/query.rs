use crate::domain::platform::{TargetArch, TargetOs};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt::Display;
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
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
    match value.to_ascii_lowercase().as_str() {
      "installer" => InstallMethod::Installer,
      _ => InstallMethod::Binary,
    }
  }
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
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
  // "auto" — each template runs its own detection at install time:
  // bash: root→/usr/local, $HOME/.local/bin→$HOME/.local, $HOME/bin→$HOME, else ./
  // ps1:  admin→ProgramFiles, LOCALAPPDATA\Programs exists→that, else ./
  "auto".to_string()
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
  "INFO".to_string()
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
    // Build directly as a Map so there is no hidden .unwrap() on the
    // Value::Object extraction that json!() + .as_object() would require.
    let mut map = Map::with_capacity(11);
    map.insert(
      "app".into(),
      Value::String(self.app.as_deref().unwrap_or("").to_string()),
    );
    map.insert("version".into(), Value::String(self.version.clone()));
    map.insert("prefix".into(), Value::String(self.prefix.clone()));
    map.insert("arch".into(), Value::String(self.arch.to_string()));
    map.insert("os".into(), Value::String(self.os.to_string()));
    map.insert("method".into(), Value::String(self.method.to_string()));
    map.insert("download_only".into(), Value::Bool(self.download_only));
    map.insert("force".into(), Value::Bool(self.force));
    map.insert("quiet".into(), Value::Bool(self.quiet));
    map.insert("log_level".into(), Value::String(self.log_level.clone()));
    map.insert("inline".into(), Value::Bool(self.inline));
    map
  }
}
