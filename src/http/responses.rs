use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use tera::escape_html;
const SCRIPT_PREVIEW_HTML_TEMPLATE: &str = include_str!("../../templates/script_preview.html");
const HIGHLIGHT_JS: &str = include_str!("../../static/css/highlightjs/highlight.min.js");
const HIGHLIGHT_CSS: &str =
  include_str!("../../static/css/highlightjs/styles/stackoverflow-dark.min.css");
const PICO_CSS: &str = include_str!("../../static/css/pico.purple.css");
const OVERRIDES_CSS: &str = include_str!("../../static/css/overrides.css");
const PAGE_CSS: &str = include_str!("../../static/css/script-preview.css");
const ICON_COPY_SVG: &str = include_str!("../../static/icons/copy.svg");
const ICON_CHECK_SVG: &str = include_str!("../../static/icons/check.svg");
const ICON_DOWNLOAD_SVG: &str = include_str!("../../static/icons/download.svg");
const CLIPBOARD_JS: &str = include_str!("../../static/clipboard.js");

/// In hot-reload mode, read `rel_path` from disk on every call (falling back
/// to the compile-time `fallback` on error).  In production, return `fallback`
/// directly — all assets are already embedded at compile time.
fn asset(rel_path: &str, fallback: &str) -> String {
  #[cfg(feature = "hot-reload")]
  {
    crate::hot_reload::read(rel_path, fallback)
  }
  #[cfg(not(feature = "hot-reload"))]
  {
    let _ = rel_path;
    fallback.to_string()
  }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ScriptResponse {
  filename: String,
  #[serde(skip)]
  shell_name: String,
  #[serde(skip)]
  inline: bool,
  #[serde(skip)]
  html: bool,
  #[serde(skip)]
  body: String,
  body_size: usize,
  #[serde(skip)]
  release_tags: Vec<String>,
}

fn sanitize_filename(name: &str) -> String {
  name
    .chars()
    .map(|c| {
      if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
        c
      } else {
        '_'
      }
    })
    .collect()
}

impl ScriptResponse {
  pub(crate) fn new(filename: String, body: String, inline: bool, html: bool) -> ScriptResponse {
    let filename = sanitize_filename(&filename);
    let body_size = body.len();
    let shell_name = match filename.split('.').next_back().unwrap() {
      "sh" => "sh",
      "ps1" => "powershell",
      _ => "sh",
    }
    .to_string();

    ScriptResponse {
      filename,
      body,
      body_size,
      shell_name,
      inline,
      html,
      release_tags: vec![],
    }
  }

  pub(crate) fn with_tags(mut self, tags: Vec<String>) -> Self {
    self.release_tags = tags;
    self
  }

  fn as_html_document(&self) -> String {
    let language = match self.shell_name.as_str() {
      "powershell" => "powershell",
      _ => "bash",
    };
    let escaped_code = escape_html(self.body.as_str());
    let escaped_filename = escape_html(self.filename.as_str());

    let template = asset(
      "templates/script_preview.html",
      SCRIPT_PREVIEW_HTML_TEMPLATE,
    );
    let highlight_js = asset("static/css/highlightjs/highlight.min.js", HIGHLIGHT_JS);
    let highlight_css = asset(
      "static/css/highlightjs/styles/stackoverflow-dark.min.css",
      HIGHLIGHT_CSS,
    );
    let shared_css = format!(
      "{}\n{}",
      asset("static/css/pico.purple.css", PICO_CSS),
      asset("static/css/overrides.css", OVERRIDES_CSS),
    );
    let page_css = asset("static/css/script-preview.css", PAGE_CSS);
    let icon_copy = asset("static/icons/copy.svg", ICON_COPY_SVG);
    let icon_check = asset("static/icons/check.svg", ICON_CHECK_SVG);
    let icon_download = asset("static/icons/download.svg", ICON_DOWNLOAD_SVG);
    let clipboard_js = asset("static/clipboard.js", CLIPBOARD_JS);

    let tags_json = serde_json::to_string(&self.release_tags).unwrap_or_else(|_| "[]".to_string());

    template
      .replace("{{title}}", escaped_filename.as_str())
      .replace("{{filename}}", escaped_filename.as_str())
      .replace("{{language}}", language)
      .replace("/*__HIGHLIGHT_JS__*/", &highlight_js)
      .replace("/*__HIGHLIGHT_CSS__*/", &highlight_css)
      .replace("/*__SHARED_CSS__*/", &shared_css)
      .replace("/*__PAGE_CSS__*/", &page_css)
      .replace("/*__CLIPBOARD_JS__*/", &clipboard_js)
      .replace("{{icon_copy}}", icon_copy.trim())
      .replace("{{icon_check}}", icon_check.trim())
      .replace("{{icon_download}}", icon_download.trim())
      .replace("{{release_tags_json}}", &tags_json)
      .replace("{{code}}", escaped_code.as_str())
  }

  pub(crate) fn render_body(&self) -> String {
    if self.html {
      self.as_html_document()
    } else {
      self.body.clone()
    }
  }
}

impl IntoResponse for ScriptResponse {
  fn into_response(self) -> Response {
    let content_type = if self.html {
      "text/html; charset=utf-8".to_string()
    } else if self.inline {
      "text/plain; charset=utf-8".to_string()
    } else {
      format!("application/x-{}", self.shell_name)
    };
    let filename = self.filename.clone();
    let body = self.render_body();

    Response::builder()
      .status(StatusCode::OK)
      .header("Content-Type", content_type)
      .header("Content-Disposition", format!("inline; filename=\"{}\"", filename))
      .body(body.into())
      .unwrap_or_else(|_| {
        Response::builder()
          .status(StatusCode::INTERNAL_SERVER_ERROR)
          .body("failed to build response".into())
          .expect("fallback response is always valid")
      })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use axum::body::to_bytes;

  #[tokio::test]
  async fn script_response_defaults_to_script_mime() {
    let response = ScriptResponse::new(
      "install-yq.sh".to_string(),
      "echo hello".to_string(),
      false,
      false,
    )
    .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
      response.headers().get("Content-Type").unwrap(),
      "application/x-sh"
    );
    assert_eq!(
      response.headers().get("Content-Disposition").unwrap(),
      "inline; filename=\"install-yq.sh\""
    );
  }

  #[tokio::test]
  async fn script_response_inline_uses_text_plain() {
    let response = ScriptResponse::new(
      "install-yq.sh".to_string(),
      "echo hello".to_string(),
      true,
      false,
    )
    .into_response();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
      response.headers().get("Content-Type").unwrap(),
      "text/plain; charset=utf-8"
    );
  }

  #[tokio::test]
  async fn script_response_html_uses_template_and_escaping() {
    let response = ScriptResponse::new(
      "install.ps1".to_string(),
      "Write-Output \"<unsafe>\"".to_string(),
      true,
      true,
    )
    .into_response();

    assert_eq!(
      response.headers().get("Content-Type").unwrap(),
      "text/html; charset=utf-8"
    );

    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    assert!(body.contains("<pre><code id=\"script-code\" class=\"language-powershell\">"));
    assert!(body.contains("&lt;unsafe&gt;"));
    assert!(body.contains("id=\"download-script\""));
  }
}
