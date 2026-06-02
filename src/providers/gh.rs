use crate::config::CONFIG;
use crate::domain::platform::TargetDeployment;
use crate::error::AppError;
use crate::supported_apps::{DownloadInfo, Repo};
use log::debug;
use moka::future::Cache;
use octocrab::models::repos::Release;
use octocrab::{Octocrab, OctocrabBuilder};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

const MIN_ASSET_SIZE: u64 = 64 * 1024; // arbitrary, may need to change if we start installing scripts

// Cache key: (owner, repo, version)
type CacheKey = (String, String, String);
type TagsCacheKey = (String, String);

static RELEASE_CACHE: LazyLock<Cache<CacheKey, Release>> = LazyLock::new(|| {
  let cache_config = &CONFIG.cache.github_releases;
  Cache::builder()
    .max_capacity(cache_config.max_capacity)
    .time_to_live(Duration::from_secs(cache_config.ttl_seconds))
    .build()
});

static TAGS_CACHE: LazyLock<Cache<TagsCacheKey, Vec<String>>> = LazyLock::new(|| {
  let cache_config = &CONFIG.cache.github_releases;
  Cache::builder()
    .max_capacity(cache_config.max_capacity)
    .time_to_live(Duration::from_secs(cache_config.ttl_seconds))
    .build()
});

static OCTOCRAB: LazyLock<Arc<Octocrab>> = LazyLock::new(|| {
  Arc::new(
    OctocrabBuilder::default()
      .build()
      .expect("Failed to build Octocrab client"),
  )
});

fn timeout_err(secs: u64) -> AppError {
  AppError::UpstreamGithub(format!("GitHub API request timed out after {} seconds", secs))
}

fn split_owner_repo(repo: &Repo) -> Result<(String, String), AppError> {
  let s = repo.get_github_repo()?;
  s.split_once('/')
    .map(|(o, r)| (o.to_string(), r.to_string()))
    .ok_or_else(|| AppError::InvalidInput(format!("Invalid github repo path: {}", s)))
}

pub(crate) async fn get_github_download_links(
  repo: &Repo,
  target_deployment: &TargetDeployment,
  version: &str,
) -> Result<Vec<DownloadInfo>, AppError> {
  let (owner, repo_name) = split_owner_repo(repo)?;

  let cache_key = (owner.clone(), repo_name.clone(), version.to_string());

  debug!("checking for release '{}' from {}/{}", version, owner, repo_name);

  let release = if let Some(cached) = RELEASE_CACHE.get(&cache_key).await {
    debug!("cache hit for {}/{} version {}", owner, repo_name, version);
    cached
  } else {
    debug!("cache miss for {}/{} version {}", owner, repo_name, version);
    let repo_handle = OCTOCRAB.repos(&owner, &repo_name);
    let releases = repo_handle.releases();

    let timeout_secs = CONFIG.github.api_timeout_seconds;
    let release = tokio::time::timeout(Duration::from_secs(timeout_secs), async {
      match version {
        "latest" => releases.get_latest().await,
        _ => releases.get_by_tag(version).await,
      }
    })
    .await
    .map_err(|_| timeout_err(timeout_secs))??;

    RELEASE_CACHE.insert(cache_key, release.clone()).await;
    release
  };

  let mut matched = vec![];
  let skippable_extensions = [
    ".asc", ".md5", ".sha1", ".sha256", ".sha512", ".sig", ".txt",
  ];
  let skippable_mimetypes = [
    mime::TEXT_PLAIN, // will probably break the ability to get scripts, so we may remove this
  ];
  let download_infos: Vec<DownloadInfo> = release
    .assets
    .iter()
    .map(DownloadInfo::from_asset)
    .collect();

  let (name_width, filetype_width, mime_width, size_width, deployment_width) =
    calc_all_widths(&download_infos);
  let col = |value: String, width: usize| format!("{value:<width$.width$}");

  for download_info in download_infos {
    let is_target = &download_info.target.deployment == target_deployment;
    let extension_skippable = skippable_extensions
      .iter()
      .any(|ext| download_info.name.ends_with(ext));
    let mimetype_skippable = skippable_mimetypes
      .iter()
      .any(|mime| download_info.content_type.essence_str() == mime.essence_str());
    let is_big_enough = download_info.size > MIN_ASSET_SIZE;
    let name_col = col(format!("{:?}", download_info.name), name_width);
    let filetype_col = col(
      format!("{:?}", download_info.target.filetype.to_string()),
      filetype_width,
    );
    let mime_col = col(
      format!("{:?}", download_info.content_type.essence_str()),
      mime_width,
    );
    let size_col = col(
      format!("{:.3}", download_info.size as f64 / (1024f64.powi(2))),
      size_width,
    );
    let deployment_col = col(
      format!("{:?}", download_info.target.deployment.to_string()),
      deployment_width,
    );
    if is_target && !extension_skippable && is_big_enough && !mimetype_skippable {
      debug!(
        "match=true  name={} filetype={} mime={} size_mb={} deployment={}",
        name_col, filetype_col, mime_col, size_col, deployment_col
      );
      matched.push(download_info);
    } else {
      debug!(
        "match=false name={} filetype={} mime={} size_mb={} deployment={} ext_skip={:?} size_ok={:?} mime_skip={:?}",
        name_col,
        filetype_col,
        mime_col,
        size_col,
        deployment_col,
        extension_skippable,
        is_big_enough,
        mimetype_skippable
      );
    }
  }
  Ok(matched)
}

pub(crate) async fn get_github_release_tags(
  repo: &Repo,
  limit: u8,
) -> Result<Vec<String>, AppError> {
  let (owner, repo_name) = split_owner_repo(repo)?;

  let cache_key = (owner.clone(), repo_name.clone());

  if let Some(cached) = TAGS_CACHE.get(&cache_key).await {
    debug!("tags cache hit for {}/{}", owner, repo_name);
    return Ok(cached);
  }

  debug!("tags cache miss for {}/{}", owner, repo_name);
  let timeout_secs = CONFIG.github.api_timeout_seconds;
  let page = tokio::time::timeout(Duration::from_secs(timeout_secs), async {
    OCTOCRAB
      .repos(&owner, &repo_name)
      .releases()
      .list()
      .per_page(limit)
      .send()
      .await
  })
  .await
  .map_err(|_| {
    AppError::UpstreamGithub(format!(
      "GitHub API request timed out after {} seconds",
      timeout_secs
    ))
  })??;

  let tags: Vec<String> = page.items.into_iter().map(|r| r.tag_name).collect();
  TAGS_CACHE.insert(cache_key, tags.clone()).await;
  Ok(tags)
}

fn calc_all_widths(infos: &[DownloadInfo]) -> (usize, usize, usize, usize, usize) {
  let w = |f: fn(&DownloadInfo) -> String| {
    infos
      .iter()
      .map(f)
      .map(|s| s.len())
      .max()
      .unwrap_or(0)
      .min(32)
  };
  (
    w(|d| format!("{:?}", d.name)),
    w(|d| format!("{:?}", d.target.filetype.to_string())),
    w(|d| format!("{:?}", d.content_type.essence_str())),
    w(|d| format!("{:.3}", d.size as f64 / (1024f64.powi(2)))),
    w(|d| format!("{:?}", d.target.deployment.to_string())),
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::platform::{TargetArch, TargetOs};
  use crate::supported_apps::Repo;
  use reqwest::Client;
  use serde_json::Value;
  use std::sync::{LazyLock, Mutex, MutexGuard};
  use std::time::Duration;
  use tokio::time::sleep;

  static API_SANITY_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

  fn lock_api_sanity_tests() -> MutexGuard<'static, ()> {
    API_SANITY_TEST_LOCK
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner())
  }

  fn is_transient_github_error(message: &str) -> bool {
    let message = message.to_ascii_lowercase();
    message.contains("rate limit")
      || message.contains("service unavailable")
      || message.contains("bad gateway")
      || message.contains("timeout")
  }

  #[tokio::test]
  #[ignore = "flaky: makes live GitHub API calls that hit rate limits"]
  #[allow(clippy::await_holding_lock)]
  async fn sanity_known_github_apps_latest_release_lookup() {
    let _guard = lock_api_sanity_tests();
    for (repo, deployment) in [
      (
        Repo::github("adam-huganir/yutc"),
        TargetDeployment::new(TargetOs::Linux, TargetArch::Amd64),
      ),
      (
        Repo::github("cli/cli"),
        TargetDeployment::new(TargetOs::Linux, TargetArch::Amd64),
      ),
      (
        Repo::github("google/go-jsonnet"),
        TargetDeployment::new(TargetOs::Linux, TargetArch::Amd64),
      ),
      (
        Repo::github("jqlang/jq"),
        TargetDeployment::new(TargetOs::Linux, TargetArch::Amd64),
      ),
      (
        Repo::github("koalaman/shellcheck"),
        TargetDeployment::new(TargetOs::Linux, TargetArch::Amd64),
      ),
      (
        Repo::github("mikefarah/yq"),
        TargetDeployment::new(TargetOs::Linux, TargetArch::Amd64),
      ),
      (
        Repo::github("mvdan/sh"),
        TargetDeployment::new(TargetOs::Mac, TargetArch::Amd64),
      ),
    ] {
      let links = {
        let mut links = None;
        let mut last_error: Option<AppError> = None;
        for attempt in 1..=3 {
          match get_github_download_links(&repo, &deployment, "latest").await {
            Ok(found_links) => {
              links = Some(found_links);
              break;
            }
            Err(AppError::UpstreamGithub(message))
              if attempt < 3 && is_transient_github_error(&message) =>
            {
              eprintln!(
                "transient github error on attempt {}/3 for '{}': {}. retrying...",
                attempt,
                repo
                  .get_github_repo()
                  .unwrap_or_else(|_| "<unknown>".to_string()),
                message
              );
              sleep(Duration::from_secs(2)).await;
            }
            Err(err) => {
              last_error = Some(err);
              break;
            }
          }
        }
        match links {
          Some(links) => links,
          None => {
            let err = last_error.unwrap_or_else(|| {
              AppError::UpstreamGithub("retries exhausted without result".to_string())
            });
            panic!(
              "known app config should resolve latest release lookup: {:?}",
              err
            );
          }
        }
      };
      assert!(
        !links.is_empty(),
        "known app '{}' should have at least one matching asset in latest release for deployment {}",
        repo
          .get_github_repo()
          .unwrap_or_else(|_| "<unknown>".to_string()),
        deployment
      );
    }
  }

  #[tokio::test]
  #[allow(clippy::await_holding_lock)]
  async fn sanity_jq_release_tag_structure() -> Result<(), AppError> {
    let _guard = lock_api_sanity_tests();
    let octocrab = OctocrabBuilder::default().build().map_err(AppError::from)?;
    let release = {
      let mut release = None;
      let mut last_error = String::new();
      for attempt in 1..=3 {
        match octocrab.repos("jqlang", "jq").releases().get_latest().await {
          Ok(found_release) => {
            release = Some(found_release);
            break;
          }
          Err(err) => {
            let message = err.to_string();
            if attempt < 3 && is_transient_github_error(&message) {
              eprintln!(
                "transient github error on attempt {}/3 for jq release check: {}. retrying...",
                attempt, message
              );
              last_error = message;
              sleep(Duration::from_secs(2)).await;
              continue;
            }
            panic!("jq release tag sanity test failed: {}", err);
          }
        }
      }
      release.unwrap_or_else(|| {
        panic!(
          "jq release tag sanity test failed after retries: {}",
          last_error
        )
      })
    };
    assert!(
      release.tag_name.starts_with("jq-"),
      "expected jq tag to start with 'jq-', got '{}'",
      release.tag_name
    );
    Ok(())
  }

  #[tokio::test]
  #[allow(clippy::await_holding_lock)]
  async fn sanity_terraform_url_exists() {
    let _guard = lock_api_sanity_tests();
    let client = Client::new();
    let checkpoint = client
      .get("https://checkpoint-api.hashicorp.com/v1/check/terraform")
      .send()
      .await
      .expect("failed to fetch terraform checkpoint metadata");
    assert!(checkpoint.status().is_success());

    let body = checkpoint
      .text()
      .await
      .expect("failed reading terraform checkpoint response body");
    let payload: Value =
      serde_json::from_str(&body).expect("failed to parse terraform checkpoint JSON");
    let version = payload
      .get("current_version")
      .and_then(Value::as_str)
      .expect("terraform checkpoint response missing current_version");

    let url = format!(
      "https://releases.hashicorp.com/terraform/{version}/terraform_{version}_linux_amd64.zip"
    );
    let response = client
      .head(&url)
      .send()
      .await
      .expect("failed to check terraform release file URL");
    assert!(
      response.status().is_success(),
      "expected terraform release file to exist at '{}', got status {}",
      url,
      response.status()
    );
  }
}
