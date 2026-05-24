use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use serde_json::to_string;

#[derive(Debug)]
pub(crate) enum AppError {
  InvalidInput(String),
  UnsupportedApp(String),
  NoMatchingAssets { repo: String, target: String },
  UpstreamGithub(String),
  OctocrabError(String),
  Template(String),
}

impl AppError {
  pub(crate) fn to_json(&self) -> String {
    let body = ErrorBody {
      error: self.code(),
      message: self.message(),
    };
    to_string(&body).unwrap_or_else(|_| {
      "{\"error\":\"unknown\",\"message\":\"serialization failure\"}".to_string()
    })
  }

  fn status_code(&self) -> StatusCode {
    match self {
      AppError::InvalidInput(_) => StatusCode::BAD_REQUEST,
      AppError::UnsupportedApp(_) => StatusCode::NOT_FOUND,
      AppError::NoMatchingAssets { .. } => StatusCode::NOT_FOUND,
      AppError::UpstreamGithub(message) => {
        if message.to_ascii_lowercase().contains("rate limit") {
          StatusCode::TOO_MANY_REQUESTS
        } else {
          StatusCode::SERVICE_UNAVAILABLE
        }
      }
      AppError::Template(_) => StatusCode::INTERNAL_SERVER_ERROR,
      AppError::OctocrabError(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
  }

  fn code(&self) -> &'static str {
    match self {
      AppError::InvalidInput(_) => "invalid_input",
      AppError::UnsupportedApp(_) => "unsupported_app",
      AppError::NoMatchingAssets { .. } => "no_matching_assets",
      AppError::UpstreamGithub(_) => "upstream_github_error",
      AppError::Template(_) => "template_error",
      AppError::OctocrabError(_) => "octocrab_error",
    }
  }

  fn message(&self) -> String {
    match self {
      AppError::InvalidInput(message) => message.clone(),
      AppError::UnsupportedApp(app) => format!("Unsupported app: {}", app),
      AppError::NoMatchingAssets { repo, target } => {
        format!(
          "No matching assets found for '{}' and target '{}'",
          repo, target
        )
      }
      AppError::UpstreamGithub(message) => message.clone(),
      AppError::Template(message) => message.clone(),
      AppError::OctocrabError(message) => message.clone(),
    }
  }
}

impl std::fmt::Display for AppError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}: {}", self.code(), self.message())
  }
}

#[derive(Serialize)]
struct ErrorBody {
  error: &'static str,
  message: String,
}

impl IntoResponse for AppError {
  fn into_response(self) -> Response {
    let status = self.status_code();
    let body = Json(ErrorBody {
      error: self.code(),
      message: self.message(),
    });
    (status, body).into_response()
  }
}

impl From<tera::Error> for AppError {
  fn from(value: tera::Error) -> Self {
    Self::Template(value.to_string())
  }
}

impl From<octocrab::Error> for AppError {
  fn from(value: octocrab::Error) -> Self {
    match value {
      octocrab::Error::GitHub { source, .. } => Self::UpstreamGithub(format!(
        "{} ({}) {}",
        source.message,
        source.status_code,
        source.documentation_url.unwrap_or("".to_string())
      )),
      other => Self::OctocrabError(format!("{:?}", other)),
    }
  }
}
