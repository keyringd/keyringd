use std::ffi::{OsStr, OsString};
use std::path::Path;

use crate::Distribution;

pub(crate) fn build_command(
    distribution: Distribution,
    xtask: &Path,
    application: &Path,
) -> Vec<OsString> {
    let format = match distribution {
        Distribution::Ubuntu => "deb",
        Distribution::Fedora => "rpm",
    };
    vec![
        xtask.as_os_str().to_owned(),
        "dist".into(),
        "--workspace".into(),
        "/workspace".into(),
        "--target".into(),
        "linux".into(),
        "--format".into(),
        format.into(),
        application.as_os_str().to_owned(),
        "--output-dir".into(),
        format!(
            "/workspace/target/linux-system/packages/{}",
            distribution.id()
        )
        .into(),
    ]
}

pub(crate) fn install_command(distribution: Distribution, package: &Path) -> Vec<OsString> {
    match distribution {
        Distribution::Ubuntu => vec![
            "dpkg".into(),
            "--install".into(),
            package.as_os_str().to_owned(),
        ],
        Distribution::Fedora => vec![
            "rpm".into(),
            "--upgrade".into(),
            "--replacepkgs".into(),
            package.as_os_str().to_owned(),
        ],
    }
}

pub(crate) fn matches(distribution: Distribution, path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .is_some_and(|name| match distribution {
            Distribution::Ubuntu => name.starts_with("keyringd_") && name.ends_with(".deb"),
            Distribution::Fedora => name.starts_with("keyringd-") && name.ends_with(".rpm"),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "build_command" {
            case "Both distributions use the release packager without a shell" {
                verify "invoke the release packaging command directly for Ubuntu and Fedora" {
                    for (distribution, format) in [(Distribution::Ubuntu, "deb"), (Distribution::Fedora, "rpm")] {
                        let command = build_command(distribution, Path::new("/workspace/custom target/xtask"), Path::new("/workspace/target/release/keyringd"));
                        assert_eq!(
                            command,
                            [
                                "/workspace/custom target/xtask",
                                "dist",
                                "--workspace",
                                "/workspace",
                                "--target",
                                "linux",
                                "--format",
                                format,
                                "/workspace/target/release/keyringd",
                                "--output-dir",
                                &format!("/workspace/target/linux-system/packages/{}", distribution.id()),
                            ]
                            .map(OsString::from)
                        );
                    }
                }
            }
        }
    }
}
