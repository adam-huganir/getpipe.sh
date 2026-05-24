use mime::Mime;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use utoipa::ToSchema;

fn get_extensions(filename: &str) -> Vec<String> {
  let parts: Vec<String> = filename.split('.').skip(1).map(|s| s.to_string()).collect();

  parts
    .iter()
    .filter(|x| !x.is_empty() && x.len() <= 4)
    .cloned()
    .collect()
}

#[derive(PartialEq, Debug, Serialize, Deserialize, ToSchema)]
pub(crate) enum ArchiveType {
  Tar,
  TarGz,
  TarBz2,
  TarXz,
  _7z,
  Zip,
  Rar,
  Gzip,
}

impl Display for ArchiveType {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      ArchiveType::Tar => write!(f, "tar"),
      ArchiveType::TarGz => write!(f, "tar.gz"),
      ArchiveType::TarBz2 => write!(f, "tar.bz2"),
      ArchiveType::TarXz => write!(f, "tar.xz"),
      ArchiveType::_7z => write!(f, "7z"),
      ArchiveType::Zip => write!(f, "zip"),
      ArchiveType::Rar => write!(f, "rar"),
      ArchiveType::Gzip => write!(f, "gz"),
    }
  }
}

impl ArchiveType {
  fn identify(input: &str) -> Option<ArchiveType> {
    let tar = ["tar"];
    let tar_gz = ["tar.gz", "tgz"];
    let tar_bz2 = ["tar.bz2"];
    let tar_xz = ["tar.xz"];
    let z7z = ["7z"];
    let rar = ["rar"];
    let gz = ["gz"];
    let zip = ["zip"];

    if tar.iter().any(|x| input.ends_with(x)) {
      return Some(ArchiveType::Tar);
    }
    if tar_gz.iter().any(|x| input.ends_with(x)) {
      return Some(ArchiveType::TarGz);
    }
    if tar_bz2.iter().any(|x| input.ends_with(x)) {
      return Some(ArchiveType::TarBz2);
    }
    if tar_xz.iter().any(|x| input.ends_with(x)) {
      return Some(ArchiveType::TarXz);
    }
    if z7z.iter().any(|x| input.ends_with(x)) {
      return Some(ArchiveType::_7z);
    }
    if zip.iter().any(|x| input.ends_with(x)) {
      return Some(ArchiveType::Zip);
    }
    if rar.iter().any(|x| input.ends_with(x)) {
      return Some(ArchiveType::Rar);
    }
    if gz.iter().any(|x| input.ends_with(x)) {
      return Some(ArchiveType::Gzip);
    }
    None
  }
}

#[derive(PartialEq, Debug, Serialize, Deserialize, ToSchema)]
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
      "exe" => Some(InstallerType::Exe),
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

#[derive(PartialEq, Debug, Serialize, Deserialize, ToSchema)]
pub(crate) enum ScriptType {
  Bat,
  Sh,
  Ps1,
  Python,
  Lua,
}

impl ScriptType {
  #[allow(dead_code)]
  fn identify(input: &str) -> Option<ScriptType> {
    let extensions = &get_extensions(input);
    if extensions.is_empty() {
      return None;
    }
    match extensions.last().unwrap().as_str() {
      "bat" => Some(ScriptType::Bat),
      "sh" => Some(ScriptType::Sh),
      "ps1" => Some(ScriptType::Ps1),
      "py" => Some(ScriptType::Python),
      "lua" => Some(ScriptType::Lua),
      _ => None,
    }
  }
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

#[derive(PartialEq, Debug, Serialize, Deserialize, ToSchema)]
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
      "application/zip" => Filetype::Archive(ArchiveType::Zip),
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
