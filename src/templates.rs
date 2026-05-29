use log::info;
use serde_json::{Map, Value, json};
use shell_quote::{Bash, Quote};
use std::collections::HashMap;
use std::sync::LazyLock;
use tera::{Filter, Tera};

pub(crate) static TEMPLATES: LazyLock<Tera> = LazyLock::new(|| {
  let mut tera = Tera::default();
  let (install_sh, content) = ("install.sh", include_str!("../templates/install.sh"));
  info!("adding template {}", install_sh);
  tera
    .add_raw_template(install_sh, content)
    .unwrap_or_else(|e| panic!("failed to add {} template: {}", install_sh, e));
  let (install_ps1, content) = ("install.ps1", include_str!("../templates/install.ps1"));
  info!("adding template {}", install_ps1);
  tera
    .add_raw_template(install_ps1, content)
    .unwrap_or_else(|e| panic!("failed to add {} template: {}", install_ps1, e));
  tera.register_filter("escape_shell", ShellEscape);
  tera.register_filter("escape_ps1", Ps1Escape);
  tera.register_filter("enumerate", Enumerate);
  tera
});

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
