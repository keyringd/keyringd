use crate::core::{model::Workspace, service::documentation};
use clap::Subcommand;

use super::PromoVideoTask;

#[derive(Subcommand)]
pub(crate) enum DocsTask {
    /// Generates static HTML in website/build/roq.
    Build {
        /// URL prefix for the generated website; defaults to the domain root.
        #[arg(long, default_value = "/", value_parser = parse_base_path)]
        base_path: String,
    },
    /// Starts Roq's development server with live reload.
    Dev {
        #[arg(long, default_value_t = 8080, value_parser = clap::value_parser!(u16).range(1..))]
        port: u16,
    },
    /// Builds documentation-site media.
    Pv {
        #[command(subcommand)]
        task: PromoVideoTask,
    },
}

fn parse_base_path(value: &str) -> Result<String, String> {
    if !value.starts_with('/')
        || value.contains("//")
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/-_".contains(&b))
    {
        return Err("use an absolute URL path containing letters, digits, /, - or _".into());
    }
    Ok(value.to_owned())
}

impl DocsTask {
    pub(crate) fn exec(self, workspace: &Workspace) -> anyhow::Result<()> {
        match self {
            Self::Build { base_path } => documentation::build_website(workspace, &base_path),
            Self::Dev { port } => documentation::serve_website(workspace, port),
            Self::Pv { task } => task.exec(workspace),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{TaskCommand, parse};
    use super::DocsTask;

    keyringd_testkit::scenario! {
        group "parse" {
            case "Accepts docs build, dev and promo-video options" {
                verify "parse the documentation commands with their respective options" {
                    let build = parse(["xtask", "docs", "build", "--base-path", "/keyringd"]).unwrap();
                    let TaskCommand::Docs {
                        task: DocsTask::Build { base_path },
                    } = build
                    else {
                        panic!("expected docs build");
                    };
                    assert_eq!(base_path, "/keyringd");

                    let dev = parse(["xtask", "docs", "dev", "--port", "8081"]).unwrap();
                    assert!(matches!(
                        dev,
                        TaskCommand::Docs {
                            task: DocsTask::Dev { port: 8081 }
                        }
                    ));

                    let promo_video = parse(["xtask", "docs", "pv", "build"]).unwrap();
                    assert!(matches!(
                        promo_video,
                        TaskCommand::Docs {
                            task: DocsTask::Pv {
                                task: super::super::PromoVideoTask::Build
                            }
                        }
                    ));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "parse" {
            case "Rejects docs paths that could be interpreted by the windows shell" {
                verify "reject documentation base paths containing shell metacharacters" {
                    for base_path in ["relative", "/site&echo", "/site%PATH%", "/site\"", "//site"] {
                        assert!(parse(["xtask", "docs", "build", "--base-path", base_path]).is_err());
                    }
                    assert!(parse(["xtask", "docs", "dev", "--port", "0"]).is_err());
                }
            }
        }
    }
}
