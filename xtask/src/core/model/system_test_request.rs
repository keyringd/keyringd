use super::HostPlatform;
use std::path::{Path, PathBuf};
#[derive(Clone, Debug)]
pub(crate) enum SystemTestRequest {
    Linux {
        distribution: String,
        package: Option<PathBuf>,
        filter: Option<String>,
    },
    Macos {
        filter: Option<String>,
        case_timeout_seconds: u64,
    },
    Windows {
        filter: Option<String>,
    },
    Wsl {
        distribution: String,
        package_directory: Option<PathBuf>,
        filter: Option<String>,
    },
}
impl SystemTestRequest {
    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::Linux { .. } => "Linux system tests",
            Self::Macos { .. } => "macOS system tests",
            Self::Windows { .. } => "Windows system tests",
            Self::Wsl { .. } => "WSL system tests",
        }
    }
    pub(crate) fn filter(&self) -> &Option<String> {
        match self {
            Self::Linux { filter, .. }
            | Self::Macos { filter, .. }
            | Self::Windows { filter }
            | Self::Wsl { filter, .. } => filter,
        }
    }
    pub(crate) fn report_directory(
        &self,
        base: &Path,
        override_directory: Option<PathBuf>,
    ) -> PathBuf {
        let directory = override_directory.unwrap_or_else(|| match self {
            Self::Linux { distribution, .. } => base.join("linux-system").join(distribution),
            Self::Macos { .. } => base.join("macos-system"),
            Self::Windows { .. } => base.join("windows-system"),
            Self::Wsl { distribution, .. } => base.join("wsl-system").join(distribution),
        });
        if self.filter().is_some() {
            directory.join("focused")
        } else {
            directory
        }
    }
    pub(crate) fn for_host(host: HostPlatform, linux: Self) -> Self {
        match host {
            HostPlatform::Linux => linux,
            HostPlatform::Macos => Self::Macos {
                filter: linux.filter().clone(),
                case_timeout_seconds: 180,
            },
            HostPlatform::Windows => Self::Windows {
                filter: linux.filter().clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "SystemTestRequest::report_directory" {
            case "Report directory separates suites distributions and focused runs" {
                verify "compute report paths for each suite and distribution with a separate focused directory" {
                    let base = Path::new("reports");
                    for filter in [None, Some("ssh_authentication".to_owned())] {
                        let request = SystemTestRequest::Wsl {
                            distribution: "fedora".to_owned(),
                            package_directory: None,
                            filter: filter.clone(),
                        };
                        let expected = if filter.is_some() {
                            base.join("wsl-system/fedora/focused")
                        } else {
                            base.join("wsl-system/fedora")
                        };
                        assert_eq!(request.report_directory(base, None), expected);
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "SystemTestRequest::report_directory" {
            case "Report directory override still separates focused runs" {
                verify "add the focused subdirectory beneath an explicitly supplied report path" {
                    for filter in [None, Some("navigation".to_owned())] {
                        let request = SystemTestRequest::Windows {
                            filter: filter.clone(),
                        };
                        let override_directory = PathBuf::from("custom-results");
                        let expected = if filter.is_some() {
                            override_directory.join("focused")
                        } else {
                            override_directory.clone()
                        };
                        assert_eq!(
                            request.report_directory(Path::new("reports"), Some(override_directory)),
                            expected
                        );
                    }
                }
            }
        }
    }
}
