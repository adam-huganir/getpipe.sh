use crate::domain::artifact::Filetype;
use crate::domain::platform::TargetDeployment;
use mime::Mime;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(PartialEq, Debug, Serialize, Deserialize, ToSchema)]
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
  fn test_assumptions() {}

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
    ];

    for case in cases {
      assert_eq!(Target::identify(&case.input, None), case.expected);
    }
  }
}
