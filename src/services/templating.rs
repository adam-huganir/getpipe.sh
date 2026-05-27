use crate::domain::platform::TargetOs;
use crate::error::AppError;
use crate::http::query::InstallQueryOptions;
use crate::supported_apps::DownloadInfo;
use crate::templates::TEMPLATES;
use serde_json::Value;
use tera::Context;

pub(crate) fn render_install_script(
  query: &InstallQueryOptions,
  links: &[DownloadInfo],
  os: &TargetOs,
) -> Result<(String, &'static str), AppError> {
  let json_links: Vec<Value> = links.iter().map(|x| x.json()).collect();
  let mut globals = query.template_globals();
  globals.insert("assets".to_string(), Value::Array(json_links));
  let tera_context = Context::from_serialize(globals)?;

  let rendered = match os {
    TargetOs::Windows => (TEMPLATES.render("install.ps1", &tera_context)?, "ps1"),
    TargetOs::Linux | TargetOs::Mac | TargetOs::Freebsd | TargetOs::Openbsd | TargetOs::Netbsd => {
      (TEMPLATES.render("install.sh", &tera_context)?, "sh")
    }
    TargetOs::Unknown => {
      return Err(AppError::InvalidInput(
        "unknown OS; set os= to one of: linux, mac, windows, freebsd, openbsd, netbsd".to_string(),
      ));
    }
  };

  Ok(rendered)
}
