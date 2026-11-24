use anyhow::{Result, bail};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HostPlatform {
    Linux,
    Macos,
    Windows,
}
impl HostPlatform {
    pub(crate) fn parse(os: &str) -> Result<Self> {
        match os {
            "linux" => Ok(Self::Linux),
            "macos" => Ok(Self::Macos),
            "windows" => Ok(Self::Windows),
            os => bail!("unsupported host OS: {os}"),
        }
    }
    pub(crate) fn application_packages(self) -> &'static [&'static str] {
        match self {
            Self::Linux => &["keyringd-linux", "keyringd-wsl"],
            Self::Macos => &["keyringd-macos"],
            Self::Windows => &["keyringd-windows"],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    keyringd_testkit::scenario! {
        group "HostPlatform::parse" {
            case "Selects both linux applications and only the native app on other hosts" {
                verify "select the Linux desktop and WSL proxy on Linux and the native binary elsewhere" {
                    assert_eq!(
                        HostPlatform::parse("linux").unwrap().application_packages(),
                        ["keyringd-linux", "keyringd-wsl"]
                    );
                    assert_eq!(
                        HostPlatform::parse("windows")
                            .unwrap()
                            .application_packages(),
                        ["keyringd-windows"]
                    );
                    assert_eq!(
                        HostPlatform::parse("macos").unwrap().application_packages(),
                        ["keyringd-macos"]
                    );
                    assert!(HostPlatform::parse("unsupported").is_err());
                }
            }
        }
    }
}
