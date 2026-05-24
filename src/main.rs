use anyhow::Context;
use axum::{
  extract::{Path, Query, Request},
  http::{
    header::{ACCEPT, CONTENT_TYPE},
    HeaderMap, HeaderValue, Method, StatusCode, Uri,
  },
  middleware::{self, Next},
  response::{Html, IntoResponse, Redirect},
  routing::get,
  Router,
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
mod cli;
mod config;
mod domain;
mod error;
mod http;
mod providers;
mod services;
mod static_site;
mod supported_apps;
mod templates;

use crate::cli::{CliInstallOutput, Commands, ScriptCommands};
use crate::domain::platform::{TargetArch, TargetOs};
use crate::error::AppError;
use crate::http::query::{InstallMethod, InstallQueryOptions};
use crate::http::responses::ScriptResponse;
use crate::services::installer;
use crate::templates::TEMPLATES;
use clap_complete::generate;
use log::{debug, info, warn};
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Instant;

const LOG_REQUESTS_SKIP_PATHS: [&str; 1] = ["/favicon.ico"];
const LATEST_API_PREFIX: &str = "/v1";
const FAVICON: &[u8] = include_bytes!("../favicon.ico");
static TERMLIBS_ROOT: LazyLock<PathBuf> =
  LazyLock::new(|| PathBuf::from(env::var("TERMLIBS_ROOT").unwrap_or("../".into())));

fn setup_logger(log_level: &str) -> Result<(), fern::InitError> {
  let log_level = log_level.to_uppercase();
  let level = match log_level.as_str() {
    "DEBUG" => log::LevelFilter::Debug,
    "INFO" => log::LevelFilter::Info,
    "WARN" => log::LevelFilter::Warn,
    "ERROR" => log::LevelFilter::Error,
    _ => log::LevelFilter::Info,
  };
  fern::Dispatch::new()
    .format(|out, message, record| {
      out.finish(format_args!(
        "{} {}: {}",
        chrono::Local::now().format("[%Y-%m-%d %H:%M:%S]"),
        record.level(),
        message
      ));
    })
    .level(level)
    .chain(std::io::stdout())
    .apply()?;
  warn!("Logging initialized at level {}", log_level);
  Ok(())
}

async fn favicon() -> impl IntoResponse {
  (
    StatusCode::OK,
    [
      ("Content-Type", "image/x-icon"),
      ("Cache-Control", "public, max-age=86400"),
    ],
    FAVICON,
  )
}

#[utoipa::path(
  get,
  path = "/install/{user}/{repo}",
  params(
    ("user" = String, Path, description = "GitHub username"),
    ("repo" = String, Path, description = "GitHub repository name"),
    ("inline" = Option<bool>, Query, description = "Return script as browser-friendly plain text when true", nullable)
  ),
  responses(
    (status = 200, description = "Install script (bash)for arbitrary GitHub repository", body = ScriptResponse, content_type = "application/x-sh"),
    (status = 200, description = "Install script (powershell) for arbitrary GitHub repository", body = ScriptResponse, content_type = "application/x-powershell")
  ),
  tag = "install"
)]
async fn install_arbitrary_github_handler(
  Path((user, repo)): Path<(String, String)>,
  Query(mut q): Query<InstallQueryOptions>,
  headers: HeaderMap,
) -> Result<ScriptResponse, AppError> {
  debug!(
    "install_arbitrary_github_handler({:?}, {:?}) with {:#?}",
    user, repo, q
  );
  installer::build_arbitrary_github_install_script(&user, &repo, &mut q, accepts_html(&headers))
    .await
}

#[utoipa::path(
  get,
  path = "/install/{app}",
  params(
    ("app" = String, Path, description = "Application name (e.g., yq, jq, gh)"),
    ("os" = Option<String>, Query, description = "target os"),
    ("arch" = Option<String>, Query, description = "target architecture", nullable),
    ("prefix" = Option<String>, Query, description = "install directory", nullable),
    ("version" = Option<String>, Query, description = "app version, default is latest", nullable),
    ("inline" = Option<bool>, Query, description = "Return script as browser-friendly plain text when true", nullable)
  ),
  responses(
    (status = 200, description = "Install script (bash) for the application", body = ScriptResponse, content_type = "application/x-sh"),
    (status = 200, description = "Install script (powershell) for the application", body = ScriptResponse, content_type = "application/x-powershell")
  ),
  tag = "install"
)]
async fn install_handler(
  Path(app): Path<String>,
  Query(mut q): Query<InstallQueryOptions>,
  headers: HeaderMap,
) -> Result<ScriptResponse, AppError> {
  debug!("install_handler({:?}, {:?})", app, q);
  installer::build_supported_install_script(&app, &mut q, accepts_html(&headers)).await
}

async fn install_latest_redirect(uri: Uri) -> Redirect {
  let path_and_query = uri
    .path_and_query()
    .map(|v| v.as_str())
    .unwrap_or(uri.path());
  Redirect::temporary(&format!("{LATEST_API_PREFIX}{path_and_query}"))
}

fn accepts_html(headers: &HeaderMap) -> bool {
  headers
    .get(ACCEPT)
    .and_then(|v| v.to_str().ok())
    .map(|v| v.contains("text/html"))
    .unwrap_or(false)
}

async fn root_handler() -> Result<Html<String>, AppError> {
  info!("{:?}", "root");
  info!("{:?}", TERMLIBS_ROOT);
  let html = static_site::load_static("index.html")
    .ok_or_else(|| AppError::InvalidInput("index.html not found".to_string()))?;
  Ok(Html(html))
}

async fn not_found_handler() -> impl IntoResponse {
  let html = static_site::load_static("404.html").unwrap_or("<h1>404 Not Found</h1>".to_string());
  (StatusCode::NOT_FOUND, Html(html))
}

fn is_truthy_env(name: &str) -> bool {
  env::var(name)
    .ok()
    .map(|value| value.trim().to_ascii_lowercase())
    .map(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
    .unwrap_or(false)
}

async fn log_requests_middleware(request: Request, next: Next) -> impl IntoResponse {
  let method = request.method().clone();
  let uri = request.uri().clone();
  if LOG_REQUESTS_SKIP_PATHS.contains(&uri.path()) {
    return next.run(request).await;
  }
  debug!("{} {} -> (received)", method, uri,);
  let started = Instant::now();
  let response = next.run(request).await;
  let status = response.status();
  let elapsed = started.elapsed();
  debug!(
    "{} {} -> {} (completed in {} ms)",
    method,
    uri,
    status.as_u16(),
    elapsed.as_millis()
  );
  response
}

#[derive(OpenApi)]
#[openapi(
  info(
    title = "Termlibs API",
    version = "0.5.0",
    description = "Terminal library installer API - Generate install scripts for popular CLI tools",
    contact(
      name = "Termlibs",
      email = "adam@huganir.com",
      url = "https://github.com/termlibs"
    )
  ),
  servers(
    (url = "http://localhost:8000/v1", description = "Local server"),
    (url = "https://termlibs.dev/v1", description = "Production server")
  ),
  paths(
    install_handler,
    install_arbitrary_github_handler
  ),
  components(
    schemas(InstallQueryOptions, ScriptResponse, InstallMethod, TargetOs, TargetArch)
  ),
  tags(
    (name = "install", description = "Install script generation")
  )
)]
struct ApiDoc;

async fn run_server(serve_args: Option<&cli::ServeArgs>) -> anyhow::Result<()> {
  // Load config (also initializes lazy static CONFIG)
  let config = &config::CONFIG;

  let log_level = serve_args
    .and_then(|args| args.log_level().map(|v| v.to_string()))
    .unwrap_or_else(|| env::var("LOG_LEVEL").unwrap_or("INFO".to_string()));

  setup_logger(log_level.as_str()).context("failed to initialize logger")?;

  // make sure the templates are loaded early to check for errors
  TEMPLATES.get_template_names().for_each(|name| {
    info!("template loaded: {}", name);
  });

  let port = serve_args
    .and_then(|args| args.port())
    .map(Ok)
    .unwrap_or_else(|| {
      env::var("PORT")
        .unwrap_or_else(|_| config.server.port.to_string())
        .parse::<u16>()
        .with_context(|| "invalid PORT value; expected u16".to_string())
    })?;

  let listen_ip: String = serve_args
    .and_then(|args| args.listen().map(|v| v.to_string()))
    .unwrap_or_else(|| env::var("LISTEN_IP").unwrap_or_else(|_| config.server.host.clone()));

  let log_requests_enabled = serve_args
    .map(|args| args.log_requests())
    .unwrap_or_else(|| is_truthy_env("LOG_REQUESTS"));

  info!("starting server at {:?}:{}", listen_ip, port);

  let app = build_app(log_requests_enabled)?;

  let addr = format!("{}:{}", listen_ip, port)
    .parse::<SocketAddr>()
    .with_context(|| format!("invalid listen address: {}:{}", listen_ip, port))?;
  let listener = tokio::net::TcpListener::bind(addr)
    .await
    .context("failed to bind TCP listener")?;

  axum::serve(listener, app)
    .await
    .context("axum server exited with error")?;
  Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let cli = cli::parse();

  match cli.command {
    Some(Commands::Script(script_cmd)) => match script_cmd {
      ScriptCommands::Install(args) => match args.run().await {
        Ok(output) => {
          let body = match output {
            CliInstallOutput::Script(response) => response.render_body(),
            CliInstallOutput::Links(json) => json,
          };
          io::stdout().write_all(body.as_bytes())?;
          Ok(())
        }
        Err(err) => {
          eprintln!("{}", err.to_json());
          std::process::exit(1);
        }
      },
    },
    Some(Commands::Install(args)) => match args.run().await {
      Ok(output) => {
        let body = match output {
          CliInstallOutput::Script(response) => response.render_body(),
          CliInstallOutput::Links(json) => json,
        };
        io::stdout().write_all(body.as_bytes())?;
        Ok(())
      }
      Err(err) => {
        eprintln!("{}", err.to_json());
        std::process::exit(1);
      }
    },
    Some(Commands::Completions(args)) => {
      let mut command = cli::build_command();
      generate(args.shell, &mut command, "termlibs", &mut io::stdout());
      Ok(())
    }
    Some(Commands::Serve(args)) => run_server(Some(&args)).await,
    None => run_server(None).await,
  }
}

fn build_cors_layer() -> anyhow::Result<CorsLayer> {
  let raw_origins = env::var("CORS_ALLOWED_ORIGINS")
    .unwrap_or_else(|_| "http://localhost:8000,https://termlibs.dev".to_string());
  let origins: Vec<HeaderValue> = raw_origins
    .split(',')
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(|s| HeaderValue::from_str(s).with_context(|| format!("invalid CORS origin: {}", s)))
    .collect::<Result<_, _>>()?;

  Ok(
    CorsLayer::new()
      .allow_methods([Method::GET])
      .allow_headers([ACCEPT, CONTENT_TYPE])
      .allow_origin(origins),
  )
}

fn build_app(log_requests_enabled: bool) -> anyhow::Result<Router> {
  let cors = build_cors_layer()?;
  let v1_router = Router::new()
    .route(
      "/install/{user}/{repo}",
      get(install_arbitrary_github_handler),
    )
    .route("/install/{app}", get(install_handler));

  let mut app = Router::new()
    .route("/", get(root_handler))
    .route("/install", get(install_latest_redirect))
    .route("/install/{app}", get(install_latest_redirect))
    .route("/install/{user}/{repo}", get(install_latest_redirect))
    .route("/favicon.ico", get(favicon))
    .nest("/v1", v1_router)
    .merge(SwaggerUi::new("/swagger-ui").url("/openapi.json", ApiDoc::openapi()))
    .fallback(not_found_handler)
    .layer(cors);

  if log_requests_enabled {
    debug!("request logging middleware enabled");
    app = app.layer(middleware::from_fn(log_requests_middleware));
  }

  Ok(app)
}

#[cfg(test)]
mod tests {
  use super::*;
  use axum_test::TestServer;

  async fn test_server() -> TestServer {
    let app = build_app(false).unwrap();
    TestServer::new(app)
  }

  #[tokio::test]
  async fn test_favicon() {
    let server = test_server().await;
    let response = server.get("/favicon.ico").await;
    response.assert_status_ok();
    response.assert_header("Content-Type", "image/x-icon");
  }

  #[tokio::test]
  async fn test_install_yutc() {
    let server = test_server().await;
    let response = server.get("/v1/install/yutc").await;
    response.assert_status_ok();
    response.assert_header("Content-Type", "application/x-sh");
  }

  #[tokio::test]
  async fn test_install_latest_redirects_to_v1_with_query() {
    let server = test_server().await;
    let response = server.get("/install/yutc?arch=amd64").await;
    response.assert_status(StatusCode::TEMPORARY_REDIRECT);
    response.assert_header("Location", "/v1/install/yutc?arch=amd64");
  }

  #[tokio::test]
  async fn test_not_found_html_page() {
    let server = test_server().await;
    let response = server.get("/definitely-not-real").await;
    response.assert_status(StatusCode::NOT_FOUND);
    response.assert_header("Content-Type", "text/html; charset=utf-8");
    response.assert_text_contains("404 not found ya doink");
  }
}
