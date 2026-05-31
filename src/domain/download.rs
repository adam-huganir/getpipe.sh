use crate::domain::artifact::Filetype;
use crate::domain::platform::TargetDeployment;
use mime::Mime;
use serde::{Deserialize, Serialize};
#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub(crate) struct Target {
  pub(crate) deployment: TargetDeployment,
  pub(crate) filetype: Filetype,
}

impl Target {
  pub(crate) fn identify(input: &str, content_type: Option<&Mime>) -> Target {
    Target {
      deployment: TargetDeployment::identify(input),
      filetype: Filetype::identify(input, content_type),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::artifact::ArchiveType;
  use crate::domain::platform::{TargetArch, TargetOs};

  struct Itc {
    input: String,
    expected: Target,
  }

  impl Itc {
    fn new(input: &str, expected: Target) -> Itc {
      Itc {
        input: input.to_string(),
        expected,
      }
    }
  }

  #[test]
  fn test_identify() {
    let cases = vec![
      Itc::new(
        "yq_darwin_amd64",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Mac,
            arch: TargetArch::Amd64,
          },
          filetype: Filetype::Binary,
        },
      ),
      Itc::new(
        "yq_darwin_amd64.tar.gz",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Mac,
            arch: TargetArch::Amd64,
          },
          filetype: Filetype::Archive(ArchiveType::TarGz),
        },
      ),
      Itc::new(
        "yq_darwin_arm64",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Mac,
            arch: TargetArch::Arm64,
          },
          filetype: Filetype::Binary,
        },
      ),
      Itc::new(
        "yq_darwin_arm64.tar.gz",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Mac,
            arch: TargetArch::Arm64,
          },
          filetype: Filetype::Archive(ArchiveType::TarGz),
        },
      ),
      Itc::new(
        "yq_freebsd_386",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Freebsd,
            arch: TargetArch::x86,
          },
          filetype: Filetype::Binary,
        },
      ),
      Itc::new(
        "yq_freebsd_386.tar.gz",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Freebsd,
            arch: TargetArch::x86,
          },
          filetype: Filetype::Archive(ArchiveType::TarGz),
        },
      ),
      Itc::new(
        "yq_freebsd_amd64",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Freebsd,
            arch: TargetArch::Amd64,
          },
          filetype: Filetype::Binary,
        },
      ),
      Itc::new(
        "yq_freebsd_amd64.tar.gz",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Freebsd,
            arch: TargetArch::Amd64,
          },
          filetype: Filetype::Archive(ArchiveType::TarGz),
        },
      ),
      Itc::new(
        "yq_freebsd_arm",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Freebsd,
            arch: TargetArch::Arm32,
          },
          filetype: Filetype::Binary,
        },
      ),
      Itc::new(
        "yq_freebsd_arm.tar.gz",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Freebsd,
            arch: TargetArch::Arm32,
          },
          filetype: Filetype::Archive(ArchiveType::TarGz),
        },
      ),
      Itc::new(
        "yq_linux_386",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Linux,
            arch: TargetArch::x86,
          },
          filetype: Filetype::Binary,
        },
      ),
      Itc::new(
        "yq_linux_386.tar.gz",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Linux,
            arch: TargetArch::x86,
          },
          filetype: Filetype::Archive(ArchiveType::TarGz),
        },
      ),
      Itc::new(
        "yq_linux_amd64",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Linux,
            arch: TargetArch::Amd64,
          },
          filetype: Filetype::Binary,
        },
      ),
      Itc::new(
        "yq_linux_amd64.tar.gz",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Linux,
            arch: TargetArch::Amd64,
          },
          filetype: Filetype::Archive(ArchiveType::TarGz),
        },
      ),
      Itc::new(
        "yq_linux_arm",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Linux,
            arch: TargetArch::Arm32,
          },
          filetype: Filetype::Binary,
        },
      ),
      Itc::new(
        "yq_linux_mips",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Linux,
            arch: TargetArch::Mips,
          },
          filetype: Filetype::Binary,
        },
      ),
      Itc::new(
        "yq_linux_mips.tar.gz",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Linux,
            arch: TargetArch::Mips,
          },
          filetype: Filetype::Archive(ArchiveType::TarGz),
        },
      ),
      Itc::new(
        "yq_linux_mips64",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Linux,
            arch: TargetArch::Mips64,
          },
          filetype: Filetype::Binary,
        },
      ),
      Itc::new(
        "yq_linux_mips64.tar.gz",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Linux,
            arch: TargetArch::Mips64,
          },
          filetype: Filetype::Archive(ArchiveType::TarGz),
        },
      ),
      Itc::new(
        "yq_linux_mips64le",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Linux,
            arch: TargetArch::Mips64Le,
          },
          filetype: Filetype::Binary,
        },
      ),
      Itc::new(
        "yq_linux_mips64le.tar.gz",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Linux,
            arch: TargetArch::Mips64Le,
          },
          filetype: Filetype::Archive(ArchiveType::TarGz),
        },
      ),
      // aarch64 (Red Hat etc. naming) must map to the same variant as arm64 (Apple naming)
      Itc::new(
        "tool_linux_aarch64.tar.gz",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Linux,
            arch: TargetArch::Arm64,
          },
          filetype: Filetype::Archive(ArchiveType::TarGz),
        },
      ),
      Itc::new(
        "tool_linux_aarch64",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Linux,
            arch: TargetArch::Arm64,
          },
          filetype: Filetype::Binary,
        },
      ),
      // uppercase asset names must be recognized (e.g. some Go projects use AMD64)
      Itc::new(
        "tool_Linux_AMD64.tar.gz",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Linux,
            arch: TargetArch::Amd64,
          },
          filetype: Filetype::Archive(ArchiveType::TarGz),
        },
      ),
      Itc::new(
        "tool_Linux_ARM64",
        Target {
          deployment: TargetDeployment {
            os: TargetOs::Linux,
            arch: TargetArch::Arm64,
          },
          filetype: Filetype::Binary,
        },
      ),
    ];

    for case in cases {
      assert_eq!(Target::identify(&case.input, None), case.expected);
    }
  }

  // Ordering regression tests — ensure substring overlaps don't cause misidentification.
  #[test]
  fn test_darwin_is_mac_not_windows() {
    // "darwin" contains the substring "win"; TargetOs must return Mac, not Windows.
    let t = Target::identify("tool_darwin_amd64", None);
    assert_eq!(t.deployment.os, TargetOs::Mac);
  }

  #[test]
  fn test_mips64le_is_not_x86() {
    // "mips64le" contains "386"-adjacent patterns; must return Mips64Le, not x86.
    let t = Target::identify("tool_linux_mips64le", None);
    assert_eq!(t.deployment.arch, TargetArch::Mips64Le);
  }

  #[test]
  fn test_tar_gz_is_not_bare_gz() {
    // ".tar.gz" ends with both "gz" and "tar.gz"; must return TarGz, not Gzip.
    let t = Target::identify("tool_linux_amd64.tar.gz", None);
    assert_eq!(t.filetype, Filetype::Archive(ArchiveType::TarGz));
  }
}
