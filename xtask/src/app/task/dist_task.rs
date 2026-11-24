use crate::core::model::{DistFormat, DistTarget, DistributionRequest};
use crate::core::{model::Workspace, service};
use clap::{Args, ValueEnum, builder::PossibleValue};
use std::path::PathBuf;

#[derive(Args, Clone, Debug)]
pub(crate) struct DistTask {
    /// Repository root as visible in the packaging environment.
    #[arg(long)]
    pub workspace: Option<PathBuf>,

    /// Distribution artifact format.
    #[arg(long)]
    pub format: DistFormat,

    /// Build an ad-hoc signed macOS app bundle for development.
    #[arg(long)]
    pub ad_hoc: bool,

    /// Distribution target.
    #[arg(long, default_value = "linux")]
    pub target: DistTarget,

    /// Output directory for the distribution artifacts.
    #[arg(long, default_value = "dist")]
    pub output_dir: PathBuf,

    /// Pre-built application binary, or the Linux proxy for the wsl target.
    pub binary: PathBuf,

    /// Windows executable (required for wsl target).
    #[arg(long)]
    pub windows_exe: Option<PathBuf>,

    /// Windows runtime directory (required for wsl target).
    #[arg(long)]
    pub windows_runtime_dir: Option<PathBuf>,
}

impl From<DistTask> for DistributionRequest {
    fn from(task: DistTask) -> Self {
        Self {
            format: task.format,
            ad_hoc: task.ad_hoc,
            target: task.target,
            output_dir: task.output_dir,
            binary: task.binary,
            windows_exe: task.windows_exe,
            windows_runtime_dir: task.windows_runtime_dir,
        }
    }
}

impl ValueEnum for DistFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Deb, Self::Rpm, Self::Msi, Self::Runtime, Self::App]
    }
    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(match self {
            Self::Deb => "deb",
            Self::Rpm => "rpm",
            Self::Msi => "msi",
            Self::Runtime => "runtime",
            Self::App => "app",
        }))
    }
}

impl ValueEnum for DistTarget {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Linux, Self::Wsl, Self::Windows, Self::Macos]
    }
    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(PossibleValue::new(match self {
            Self::Linux => "linux",
            Self::Wsl => "wsl",
            Self::Windows => "windows",
            Self::Macos => "macos",
        }))
    }
}

impl DistTask {
    pub(crate) fn exec(self, workspace: &Workspace) -> anyhow::Result<()> {
        service::dist::run(workspace, &self.into())
    }
}

#[cfg(test)]
mod tests {
    use super::super::{TaskCommand, parse};

    keyringd_testkit::scenario! {
        group "parse" {
            case "Dist accepts macos bundles and notarized archives" {
                verify "parse macOS application distribution" {
                    for ad_hoc in [false, true] {
                        let cli = if ad_hoc {
                            parse(["xtask", "dist", "--target", "macos", "--format", "app", "--ad-hoc", "target/release/keyringd-macos"])
                        } else {
                            parse(["xtask", "dist", "--target", "macos", "--format", "app", "target/release/keyringd-macos"])
                        }.unwrap();
                        let TaskCommand::Dist(task) = cli else {
                            panic!("expected dist");
                        };
                        assert!(matches!(task.target, crate::core::model::DistTarget::Macos));
                        assert_eq!(task.ad_hoc, ad_hoc);
                        assert_eq!(
                            task.binary,
                            std::path::PathBuf::from("target/release/keyringd-macos")
                        );
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "parse" {
            case "Dist accepts the workspace visible inside the test container" {
                verify "parse the workspace path passed through to the test container" {
                    let cli = parse([
                        "xtask",
                        "dist",
                        "--workspace",
                        "/workspace",
                        "--format",
                        "deb",
                        "--target",
                        "linux",
                        "/workspace/target/release/keyringd",
                    ])
                    .unwrap();
                    let TaskCommand::Dist(task) = cli else {
                        panic!("expected dist");
                    };
                    assert_eq!(task.workspace, Some("/workspace".into()));
                }
            }
        }
    }
}
