use paste::paste;
use serde::de::{Error, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::Display;

macro_rules! impl_caseless_deserialize {
    // Two-arg form: $description is shown in error messages (e.g. in a 400 response body).
    ($enum_type:ident, $description:literal) => {
        paste! {
            struct [<$enum_type Visitor>];

            impl<'de> Visitor<'de> for [<$enum_type Visitor>] {
                type Value = $enum_type;

                fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
                    formatter.write_str($description)
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: Error,
                {
                    let result = $enum_type::identify(v);
                    if result == $enum_type::Unknown {
                        return Err(E::invalid_value(serde::de::Unexpected::Str(v), &self));
                    }
                    Ok(result)
                }
            }

            impl<'de> Deserialize<'de> for $enum_type {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    deserializer.deserialize_str([<$enum_type Visitor>])
                }
            }
        }
    };
}

#[derive(PartialEq, Debug, Clone, Serialize)]
pub(crate) enum TargetOs {
  Windows,
  Linux,
  Mac,
  Freebsd,
  Openbsd,
  Netbsd,
  Unknown,
}

impl_caseless_deserialize!(
  TargetOs,
  "a recognized OS name: linux, mac, windows, freebsd, openbsd, netbsd"
);

impl From<&str> for TargetOs {
  fn from(input: &str) -> Self {
    TargetOs::identify(input)
  }
}

impl TargetOs {
  pub(crate) fn identify(input: &str) -> TargetOs {
    let normed_input = input.to_lowercase();
    let win = ["win", "windows"];
    let linux = ["linux"];
    // "darwin" contains the substring "win"; mac MUST be checked before windows.
    let mac = ["mac", "macos", "osx", "darwin"];
    let freebsd = ["freebsd"];
    let openbsd = ["openbsd"];
    let netbsd = ["netbsd"];

    if freebsd.iter().any(|x| normed_input.contains(x)) {
      return TargetOs::Freebsd;
    }
    if openbsd.iter().any(|x| normed_input.contains(x)) {
      return TargetOs::Openbsd;
    }
    if netbsd.iter().any(|x| normed_input.contains(x)) {
      return TargetOs::Netbsd;
    }
    // Check mac before windows — "darwin" contains "win" as a substring.
    if mac.iter().any(|x| normed_input.contains(x)) {
      return TargetOs::Mac;
    }
    if win.iter().any(|x| normed_input.contains(x)) {
      return TargetOs::Windows;
    }
    if linux.iter().any(|x| normed_input.contains(x)) {
      return TargetOs::Linux;
    }
    TargetOs::Unknown
  }
}

impl Display for TargetOs {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      TargetOs::Windows => write!(f, "windows"),
      TargetOs::Linux => write!(f, "linux"),
      TargetOs::Mac => write!(f, "mac"),
      TargetOs::Freebsd => write!(f, "freebsd"),
      TargetOs::Openbsd => write!(f, "openbsd"),
      TargetOs::Netbsd => write!(f, "netbsd"),
      TargetOs::Unknown => write!(f, "unknown"),
    }
  }
}

#[derive(PartialEq, Debug, Serialize, Clone)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
pub(crate) enum TargetArch {
  Amd64,
  /// Covers both `arm64` (Apple/Linux naming) and `aarch64` (Red Had/Fedora/Others).
  Arm64,
  PPCLe,
  PPC,
  Arm32,
  MipsLe,
  Mips,
  Mips64Le,
  Mips64,
  RiscV,
  #[allow(non_camel_case_types)]
  x86,
  Unknown,
}

impl_caseless_deserialize!(
  TargetArch,
  "a recognized architecture name: amd64, arm64, x86, arm, mips64le, mips64, mipsle, mips, ppc64le, ppc64, riscv"
);

impl From<&str> for TargetArch {
  fn from(value: &str) -> Self {
    TargetArch::identify(value)
  }
}

impl Display for TargetArch {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      TargetArch::Amd64 => write!(f, "amd64"),
      TargetArch::Arm64 => write!(f, "arm64"),
      TargetArch::PPC => write!(f, "ppc64"),
      TargetArch::PPCLe => write!(f, "ppc64le"),
      TargetArch::Arm32 => write!(f, "arm"),
      TargetArch::Mips => write!(f, "mips"),
      TargetArch::MipsLe => write!(f, "mipsle"),
      TargetArch::Mips64 => write!(f, "mips64"),
      TargetArch::Mips64Le => write!(f, "mips64le"),
      TargetArch::RiscV => write!(f, "riscv"),
      TargetArch::x86 => write!(f, "x86"),
      TargetArch::Unknown => write!(f, "unknown"),
    }
  }
}

impl TargetArch {
  pub(crate) fn identify(input: &str) -> TargetArch {
    let input = input.to_lowercase();
    let amd = ["amd64", "x64", "x86_64"];
    // "386" is in x86 but is a substring of e.g. "i386"; checked after all mips variants.
    let x86 = ["x86", "i386", "i686", "x86_32", "386", "686", "ia32"];
    // "arm64"/"aarch64" checked before "arm" — "arm" is a substring of "arm64".
    let arm64 = ["arm64", "aarch64"];
    let arm32 = ["arm"];
    let ppcle = ["ppc64le", "ppc64el", "ppcle"];
    // "ppc" checked after "ppc64le"/"ppc64el" — "ppc" is a substring of "ppc64".
    let ppc = ["ppc", "ppc64", "powerpc"];
    // mips variants checked most-specific first: mips64le → mips64 → mipsle → mips.
    let mips64le = ["mips64le"];
    let mips64 = ["mips64"];
    let mipsle = ["mipsle", "mipsel"];
    let mips = ["mips"];
    let riscv = ["riscv"];

    if amd.iter().any(|x| input.contains(x)) {
      return TargetArch::Amd64;
    }
    if arm64.iter().any(|x| input.contains(x)) {
      return TargetArch::Arm64;
    }
    if ppcle.iter().any(|x| input.contains(x)) {
      return TargetArch::PPCLe;
    }
    if ppc.iter().any(|x| input.contains(x)) {
      return TargetArch::PPC;
    }
    if mips64le.iter().any(|x| input.contains(x)) {
      return TargetArch::Mips64Le;
    }
    if mips64.iter().any(|x| input.contains(x)) {
      return TargetArch::Mips64;
    }
    if mipsle.iter().any(|x| input.contains(x)) {
      return TargetArch::MipsLe;
    }
    if mips.iter().any(|x| input.contains(x)) {
      return TargetArch::Mips;
    }
    if x86.iter().any(|x| input.contains(x)) {
      return TargetArch::x86;
    }
    if arm32.iter().any(|x| input.contains(x)) {
      return TargetArch::Arm32;
    }
    if riscv.iter().any(|x| input.contains(x)) {
      return TargetArch::RiscV;
    }

    let amd64_os = ["windows64", "win64", "winx64", "linux64"];
    let x86_os = ["windows32", "win32", "winx86", "linux32"];
    if amd64_os.iter().any(|x| input.contains(x)) {
      return TargetArch::Amd64;
    }
    if x86_os.iter().any(|x| input.contains(x)) {
      return TargetArch::x86;
    }

    TargetArch::Unknown
  }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TargetDeployment {
  pub(crate) os: TargetOs,
  pub(crate) arch: TargetArch,
}

impl Display for TargetDeployment {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}-{}", self.os, self.arch)
  }
}

impl TargetDeployment {
  pub(crate) fn new(os: TargetOs, arch: TargetArch) -> TargetDeployment {
    TargetDeployment { os, arch }
  }

  pub(crate) fn identify(input: &str) -> TargetDeployment {
    TargetDeployment {
      os: TargetOs::identify(input),
      arch: TargetArch::identify(input),
    }
  }
}

impl Default for TargetDeployment {
  fn default() -> Self {
    TargetDeployment {
      os: TargetOs::Linux,
      arch: TargetArch::Amd64,
    }
  }
}
