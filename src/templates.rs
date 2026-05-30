use log::info;
use serde_json::{Map, Value, json};
use shell_quote::{Bash, Quote};
use std::collections::HashMap;
use std::sync::LazyLock;
use tera::{Filter, Tera};

// ---------------------------------------------------------------------------
// Template file manifest — name → path relative to GETPIPE_ROOT.
// Used by both the embedded (include_str!) and hot-reload paths.
// ---------------------------------------------------------------------------
const TEMPLATE_FILES: [(&str, &str); 3] = [
  ("install.sh", "templates/install.sh"),
  ("install.ps1", "templates/install.ps1"),
  ("install-help.md", "templates/install-help.md"),
];


// ---------------------------------------------------------------------------
// Build a Tera engine with all filters registered.
// The templates themselves are loaded by the caller.
// ---------------------------------------------------------------------------
fn build_tera_engine() -> Tera {
  let mut tera = Tera::default();
  tera.register_filter("escape_shell", ShellEscape);
  tera.register_filter("escape_ps1", Ps1Escape);
  tera.register_filter("enumerate", Enumerate);
  tera
}

// ---------------------------------------------------------------------------
// Production path — templates embedded at compile time via include_str!.
// ---------------------------------------------------------------------------
#[cfg(not(feature = "hot-reload"))]
static TEMPLATES: LazyLock<Tera> = LazyLock::new(|| {
  let mut tera = build_tera_engine();
  for (name, _path) in TEMPLATE_FILES {
    // include_str! is resolved at compile time; paths are matched by position.
    let content = match name {
      "install.sh" => include_str!("../templates/install.sh"),
      "install.ps1" => include_str!("../templates/install.ps1"),
      "install-help.md" => include_str!("../templates/install-help.md"),
      other => panic!("TEMPLATE_FILES contains unhandled name: {}", other),
    };
    info!("loading template: {}", name);
    tera
      .add_raw_template(name, content)
      .unwrap_or_else(|e| panic!("failed to add {} template: {}", name, e));
  }
  tera
});

// ---------------------------------------------------------------------------
// Hot-reload path — templates read from disk on every render call.
// ---------------------------------------------------------------------------
#[cfg(feature = "hot-reload")]
static TEMPLATES: LazyLock<std::sync::Mutex<Tera>> = LazyLock::new(|| {
  // In hot-reload mode every render() call re-reads from disk anyway, so
  // the initial load is best-effort: missing files are warned, not fatal.
  // This lets tests initialize the LazyLock before setting GETPIPE_ROOT.
  let mut tera = build_tera_engine();
  let root = crate::hot_reload::root();
  for (name, rel_path) in TEMPLATE_FILES {
    let path = root.join(rel_path);
    match std::fs::read_to_string(&path) {
      Ok(content) => {
        info!("hot-reload: loading template {} from {}", name, path.display());
        if let Err(e) = tera.add_raw_template(name, &content) {
          log::warn!("hot-reload: failed to parse template {}: {}", name, e);
        }
      }
      Err(e) => {
        log::warn!(
          "hot-reload: template {} not found at {} ({}); will retry on first render",
          name,
          path.display(),
          e
        );
      }
    }
  }
  std::sync::Mutex::new(tera)
});

// ---------------------------------------------------------------------------
// Public API — same surface regardless of feature flag.
// ---------------------------------------------------------------------------

/// Renders a template by name with the given context.
pub(crate) fn render(name: &str, context: &tera::Context) -> Result<String, tera::Error> {
  #[cfg(not(feature = "hot-reload"))]
  {
    TEMPLATES.render(name, context)
  }
  #[cfg(feature = "hot-reload")]
  {
    let mut tera = TEMPLATES.lock().unwrap();
    let root = crate::hot_reload::root();
    for (template_name, rel_path) in TEMPLATE_FILES {
      let path = root.join(rel_path);
      match std::fs::read_to_string(&path) {
        Ok(content) => {
          if let Err(e) = tera.add_raw_template(template_name, &content) {
            log::warn!(
              "hot-reload: failed to reload template {}: {}",
              template_name,
              e
            );
          }
        }
        Err(e) => {
          log::warn!(
            "hot-reload: failed to read {}: {}",
            path.display(),
            e
          );
        }
      }
    }
    tera.render(name, context)
  }
}

/// Returns the names of all currently loaded templates (used for startup logging).
pub(crate) fn names() -> Vec<String> {
  #[cfg(not(feature = "hot-reload"))]
  {
    TEMPLATES.get_template_names().map(String::from).collect()
  }
  #[cfg(feature = "hot-reload")]
  {
    TEMPLATES
      .lock()
      .unwrap()
      .get_template_names()
      .map(String::from)
      .collect()
  }
}

// ---------------------------------------------------------------------------
// Tera filter implementations
// ---------------------------------------------------------------------------

struct ShellEscape;

impl Filter for ShellEscape {
  fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = match value {
      Value::String(s) => s.clone(),
      Value::Bool(b) => b.to_string(),
      Value::Number(n) => n.to_string(),
      _ => String::new(),
    };
    let escaped: Vec<u8> = Bash::quote(&s);
    Ok(Value::String(String::from_utf8(escaped).unwrap()))
  }
}

/// PowerShell-specific escaping filter.
///
/// Wraps the value in single quotes and doubles any embedded single quotes.
/// Single-quoted strings in PowerShell are completely literal — no variable
/// expansion, no escape sequences — making this the safest general-purpose
/// quoting strategy.
///
/// Booleans and numbers are converted to their string representations first
/// so that `{{ force | escape_ps1 }}` with `force = false` produces `'false'`,
/// which compares correctly with PowerShell's `-eq 'true'` checks.
struct Ps1Escape;

impl Filter for Ps1Escape {
  fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = match value {
      Value::String(s) => s.clone(),
      Value::Bool(b) => b.to_string(),
      Value::Number(n) => n.to_string(),
      _ => String::new(),
    };
    // Wrap in single quotes; escape embedded single quotes by doubling them.
    let escaped = format!("'{}'", s.replace('\'', "''"));
    Ok(Value::String(escaped))
  }
}

struct Enumerate;

impl Filter for Enumerate {
  fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    if let Some(list) = value.as_array() {
      let mut result: Vec<Value> = Vec::with_capacity(list.len());
      for (i, item) in list.iter().enumerate() {
        let mut map: Map<String, Value> = Map::new();
        map.insert("index".into(), json!(i));
        map.insert("item".into(), item.clone());
        result.push(Value::Object(map));
      }
      Ok(Value::Array(result))
    } else {
      Ok(Value::Array(Vec::new()))
    }
  }
}

// ---------------------------------------------------------------------------
// Hot-reload integration test
// ---------------------------------------------------------------------------
// Runs only when compiled with --features hot-reload.
// Verifies that render() re-reads the template file from disk on every call
// so that edits are picked up without restarting the server.
//
// NOTE: this test mutates GETPIPE_ROOT via set_var.  Run with
//   --test-threads=1
// if other tests in the binary also touch that env var.
#[cfg(all(test, feature = "hot-reload"))]
mod hot_reload_tests {
  use super::*;
  use std::sync::Mutex;

  // Serialize env-var access across any concurrent test threads.
  static ENV_LOCK: Mutex<()> = Mutex::new(());

  // RAII guard: restores GETPIPE_ROOT to its previous value on drop,
  // including when the test panics, so other tests are not affected.
  struct EnvGuard {
    key: &'static str,
    prev: Option<String>,
  }

  impl EnvGuard {
    fn set(key: &'static str, val: &std::path::Path) -> Self {
      let prev = std::env::var(key).ok();
      // Safety: serialized by ENV_LOCK held by the caller.
      unsafe { std::env::set_var(key, val) };
      Self { key, prev }
    }
  }

  impl Drop for EnvGuard {
    fn drop(&mut self) {
      // Safety: serialized by ENV_LOCK held by the caller's scope.
      unsafe {
        match &self.prev {
          Some(v) => std::env::set_var(self.key, v),
          None => std::env::remove_var(self.key),
        }
      }
    }
  }

  #[test]
  fn render_picks_up_template_changes_on_disk() {
    let _env_guard = ENV_LOCK.lock().unwrap();

    // Build a temp directory that looks like a minimal GETPIPE_ROOT.
    let tmp = std::env::temp_dir().join(format!(
      "getpipe-hot-reload-test-{}",
      std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
    ));
    let templates_dir = tmp.join("templates");
    std::fs::create_dir_all(&templates_dir).expect("create temp templates dir");

    // Stub the other two templates that render() re-reads on every call.
    std::fs::write(templates_dir.join("install.sh"), "#!/bin/bash\n")
      .expect("write install.sh stub");
    std::fs::write(templates_dir.join("install.ps1"), "# stub\n")
      .expect("write install.ps1 stub");

    // Use a simple template with no variables so no context setup is needed.
    let help_path = templates_dir.join("install-help.md");
    std::fs::write(&help_path, "sentinel-v1").expect("write v1 template");

    // Point the server at our temp directory.
    // EnvGuard restores GETPIPE_ROOT even if an assert below panics.
    let _root_guard = EnvGuard::set("GETPIPE_ROOT", &tmp);

    let ctx = tera::Context::new();
    let out1 = render("install-help.md", &ctx).expect("render v1");
    assert!(
      out1.contains("sentinel-v1"),
      "expected 'sentinel-v1' in first render, got: {out1}"
    );

    // Overwrite the template on disk — next render() must pick it up.
    std::fs::write(&help_path, "sentinel-v2").expect("write v2 template");

    let out2 = render("install-help.md", &ctx).expect("render v2");
    assert!(
      out2.contains("sentinel-v2"),
      "hot-reload did not pick up change; got: {out2}"
    );
    assert!(
      !out2.contains("sentinel-v1"),
      "stale v1 content still present in: {out2}"
    );

    // Temp dir cleanup (best-effort; EnvGuard already reset GETPIPE_ROOT).
    std::fs::remove_dir_all(&tmp).ok();
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn test_shell_escape() {
    let to_quote = "
        ```bash
        sudo apt-get update
        sudo apt-get install -y curl
        ```
        ";
    let escaped = String::from_utf8(Bash::quote(to_quote)).unwrap();
    let result = ShellEscape
      .filter(&json! {to_quote}, &HashMap::new())
      .unwrap();
    println!("{:?}", result);
    assert_eq!(result.as_str().unwrap(), escaped);
  }

  #[test]
  fn test_bash_quote() {
    let demo_template = "
        From: {{ test }}
        To: {{ test | escape_shell }}
        ";
    let demo_context = tera::Context::from_value(json! {{"test":"${not a var!!}"}}).unwrap();
    let mut tera = Tera::default();
    tera.add_raw_template("demo", demo_template).unwrap();
    tera.register_filter("escape_shell", ShellEscape);
    let out = tera.render("demo", &demo_context).unwrap();
    let expected = "From: ${not a var!!}\n        To: $'${not a var!!}'";
    assert_eq!(out.trim(), expected);
  }
}
