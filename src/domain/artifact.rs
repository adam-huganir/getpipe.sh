use mime::Mime;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
fn get_extensions(filename: &str) -> Vec<String> {
  // Split on '.', skip the basename (everything before the first dot).
  // Filter only empty segments, which arise from leading dots (e.g. ".bashrc")
  // or consecutive dots.  No length cap — extensions like "sha256" or "jsonl"
  // are valid and must not be silently dropped.
  filename
    .split('.')
    .skip(1)
    .filter(|x| !x.is_empty())
    .map(|s| s.to_string())
    .collect()
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub(crate) enum ArchiveType {
  Tar,
  TarGz,
  TarBz2,
  TarXz,
  _7z,
  Rar,
  Gzip,
  Zip,
}

impl Display for ArchiveType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ArchiveType::Tar => write!(f, "tar"),
      ArchiveType::TarGz => write!(f, "tar.gz"),
      ArchiveType::TarBz2 => write!(f, "tar.bz2"),
      ArchiveType::TarXz => write!(f, "tar.xz"),
      ArchiveType::_7z => write!(f, "7z"),
      ArchiveType::Rar => write!(f, "rar"),
      ArchiveType::Gzip => write!(f, "gz"),
      ArchiveType::Zip => write!(f, "zip"),
    }
  }
}

impl ArchiveType {
  fn identify(input: &str) -> Option<ArchiveType> {
    // More-specific extensions before less-specific suffixes of them.
    if input.ends_with("tar.gz") || input.ends_with("tgz") {
      return Some(ArchiveType::TarGz);
    }
    if input.ends_with("tar.bz2") {
      return Some(ArchiveType::TarBz2);
    }
    if input.ends_with("tar.xz") {
      return Some(ArchiveType::TarXz);
    }
    if input.ends_with("tar") {
      return Some(ArchiveType::Tar);
    }
    // bare ".gz" after "tar.gz" — ".tar.gz" also ends_with ".gz"
    if input.ends_with("gz") {
      return Some(ArchiveType::Gzip);
    }
    if input.ends_with("7z") {
      return Some(ArchiveType::_7z);
    }
    if input.ends_with("rar") {
      return Some(ArchiveType::Rar);
    }
    if input.ends_with("zip") {
      return Some(ArchiveType::Zip);
    }
    None
  }
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub(crate) enum InstallerType {
  Msi,
  Exe,
  Deb,
  Rpm,
  Pkg,
}

impl InstallerType {
  fn identify(input: &str) -> Option<InstallerType> {
    let extensions = &get_extensions(input);
    if extensions.is_empty() {
      return None;
    }
    match extensions.last().unwrap().as_str() {
      "msi" => Some(InstallerType::Msi),
      "deb" => Some(InstallerType::Deb),
      "rpm" => Some(InstallerType::Rpm),
      "pkg" => Some(InstallerType::Pkg),
      _ => None,
    }
  }
}

impl Display for InstallerType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      InstallerType::Msi => write!(f, "msi"),
      InstallerType::Exe => write!(f, "exe"),
      InstallerType::Deb => write!(f, "deb"),
      InstallerType::Rpm => write!(f, "rpm"),
      InstallerType::Pkg => write!(f, "pkg"),
    }
  }
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub(crate) enum ScriptType {
  Bat,
  Sh,
  Ps1,
  Python,
  Lua,
}

impl Display for ScriptType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ScriptType::Bat => write!(f, "bat"),
      ScriptType::Sh => write!(f, "sh"),
      ScriptType::Ps1 => write!(f, "ps1"),
      ScriptType::Python => write!(f, "py"),
      ScriptType::Lua => write!(f, "lua"),
    }
  }
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub(crate) enum Filetype {
  Binary,
  Script(ScriptType),
  Installer(InstallerType),
  Archive(ArchiveType),
  Unknown,
}

impl Display for Filetype {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Filetype::Binary => write!(f, "binary"),
      Filetype::Archive(x) => write!(f, "{}", x),
      Filetype::Installer(x) => write!(f, "{} installer", x),
      Filetype::Script(x) => write!(f, "{} script", x),
      Filetype::Unknown => write!(f, "unknown"),
    }
  }
}

impl Filetype {
  fn content_type_lookup(input: &Mime) -> Filetype {
    let mime_type = input.essence_str();
    match mime_type {
      "application/x-debian-package" => Filetype::Installer(InstallerType::Deb),
      "application/x-rpm" => Filetype::Installer(InstallerType::Rpm),
      "application/x-msi" => Filetype::Installer(InstallerType::Msi),
      "application/x-xar" => Filetype::Installer(InstallerType::Pkg),
      "application/x-gtar" | "application/gzip" => Filetype::Archive(ArchiveType::TarGz),
      "application/x-ms-dos-executable" => Filetype::Binary,
      "application/x-ms-installer" => Filetype::Installer(InstallerType::Exe),
      "application/x-sh" => Filetype::Script(ScriptType::Sh),
      _ => Filetype::Unknown,
    }
  }

  pub(crate) fn identify(input: &str, content_type: Option<&Mime>) -> Filetype {
    if let Some(content_type) = content_type {
      let parsed = Self::content_type_lookup(content_type);
      if parsed != Filetype::Unknown {
        return parsed;
      }
    }

    if let Some(archive) = ArchiveType::identify(input) {
      return Filetype::Archive(archive);
    }

    if let Some(installer_type) = InstallerType::identify(input) {
      return Filetype::Installer(installer_type);
    }

    let extensions = get_extensions(input);
    if extensions.is_empty()
      || matches!(
        extensions.last().unwrap_or(&String::default()).as_str(),
        "exe"
      )
    {
      Filetype::Binary
    } else {
      Filetype::Unknown
    }
  }
}
