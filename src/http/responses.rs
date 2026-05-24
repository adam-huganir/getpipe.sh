use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use tera::escape_html;
use utoipa::ToSchema;

const SCRIPT_PREVIEW_HTML_TEMPLATE: &str = include_str!("../../templates/script_preview.html");
const HIGHLIGHT_JS: &str = include_str!("../../templates/vendor/highlightjs/highlight.min.js");
const HIGHLIGHT_CSS: &str = include_str!("../../templates/vendor/highlightjs/github-dark.min.css");

#[derive(Serialize, Deserialize, ToSchema)]
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
}

impl ScriptResponse {
  pub(crate) fn new(filename: String, body: String, inline: bool, html: bool) -> ScriptResponse {
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
    }
  }

  fn as_html_document(&self) -> String {
    let language = match self.shell_name.as_str() {
      "powershell" => "powershell",
      _ => "bash",
    };
    let escaped_code = escape_html(self.body.as_str());
    let escaped_filename = escape_html(self.filename.as_str());
    SCRIPT_PREVIEW_HTML_TEMPLATE
      .replace("{{title}}", escaped_filename.as_str())
      .replace("{{filename}}", escaped_filename.as_str())
      .replace("{{language}}", language)
      .replace("/*__HIGHLIGHT_JS__*/", HIGHLIGHT_JS)
      .replace("/*__HIGHLIGHT_CSS__*/", HIGHLIGHT_CSS)
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
    let body = if self.html {
      self.as_html_document()
    } else {
      self.body
    };

    Response::builder()
      .status(StatusCode::OK)
      .header("Content-Type", content_type)
      .header(
        "Content-Disposition",
        format!("inline; filename=\"{}\"", self.filename),
      )
      .body(body.into())
      .unwrap()
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
    assert!(body.contains("width: min(98vw, 1800px);"));
  }
}
