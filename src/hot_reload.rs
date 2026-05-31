//! Development hot-reload helpers — only compiled with `--features hot-reload`.
//!
//! Provides `root()` (resolves GETPIPE_ROOT) and `read()` (disk read with
//! compile-time fallback) used by `static_site`, `templates`, and
//! `http::responses` to pick up file changes without restarting the server.

/// Returns the project root directory, resolved from `GETPIPE_ROOT` env var.
/// Defaults to `"."` so the dev server can be started from the project root.
pub(crate) fn root() -> std::path::PathBuf {
  std::path::PathBuf::from(std::env::var("GETPIPE_ROOT").unwrap_or(".".into()))
}

/// Read `rel_path` (relative to `root()`) from disk.
/// Returns `fallback` and logs a warning if the file cannot be read.
pub(crate) fn read(rel_path: &str, fallback: &str) -> String {
  let path = root().join(rel_path);
  std::fs::read_to_string(&path).unwrap_or_else(|e| {
    log::warn!("hot-reload: failed to read {}: {}", path.display(), e);
    fallback.to_string()
  })
}
