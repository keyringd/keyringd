use crate::core::model::Workspace;
use anyhow::{Context, Result};
use semver::Version;
use std::path::Path;
use toml_edit::{DocumentMut, value};

pub(crate) fn read_version(workspace: &Workspace) -> Result<Version> {
    let cargo_toml = workspace.root().join("Cargo.toml");
    let content = std::fs::read_to_string(&cargo_toml)
        .with_context(|| format!("cannot read {}", cargo_toml.display()))?;
    let manifest: toml_edit::DocumentMut = content.parse().context("invalid workspace manifest")?;
    let version = manifest["workspace"]["package"]["version"]
        .as_str()
        .context("workspace.package.version is missing")?;
    Version::parse(version).with_context(|| {
        format!(
            "invalid workspace.package.version in {}",
            cargo_toml.display()
        )
    })
}

pub(crate) fn write_version(root: &Path, version: &Version) -> Result<()> {
    let path = root.join("Cargo.toml");
    let mut manifest: DocumentMut = std::fs::read_to_string(&path)?.parse()?;
    manifest["workspace"]["package"]["version"]
        .as_str()
        .context("workspace.package.version is missing")?;
    manifest["workspace"]["package"]["version"] = value(version.to_string());
    std::fs::write(path, manifest.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::cargo::refresh_lockfile;
    use super::*;
    use std::process::Command;

    keyringd_testkit::scenario! {
        group "read_version" {
            case "Rejects invalid versions when reading the manifest" {
                verify "return a parse failure for an invalid workspace version" {
                    let root = tempfile::tempdir().unwrap();
                    let workspace = Workspace::new(root.path().to_owned());
                    for version in ["1.2", "../invalid"] {
                        std::fs::write(
                            root.path().join("Cargo.toml"),
                            format!("[workspace.package]\nversion = '{version}'\n"),
                        )
                        .unwrap();
                        let error = read_version(&workspace).unwrap_err();
                        assert!(
                            error
                                .to_string()
                                .contains("invalid workspace.package.version")
                        );
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "write_version" {
            case "Changed workspace version updates every package in a usable lockfile" {
                verify "update all local package entries and retain a readable Cargo lockfile" {
                    let root = tempfile::tempdir().unwrap();
                    std::fs::write(
                                root.path().join("Cargo.toml"),
                                "[workspace]\nresolver = '2'\nmembers = ['keyringd-core', 'xtask', 'testkit']\n[workspace.package]\nversion = '0.6.0'\nedition = '2024'\n",
                            )
                            .unwrap();
                    for package in ["keyringd-core", "xtask", "testkit"] {
                        let directory = root.path().join(package);
                        std::fs::create_dir_all(directory.join("src")).unwrap();
                        std::fs::write(directory.join("src/lib.rs"), "").unwrap();
                        std::fs::write(
                                    directory.join("Cargo.toml"),
                                    format!("[package]\nname = '{package}'\nversion.workspace = true\nedition.workspace = true\n"),
                                )
                                .unwrap();
                    }
                    let cargo = std::env::var_os("CARGO")
                        .map(std::path::PathBuf::from)
                        .unwrap_or_else(|| "cargo".into());
                    assert!(
                        Command::new(&cargo)
                            .args(["generate-lockfile", "--offline"])
                            .current_dir(root.path())
                            .status()
                            .unwrap()
                            .success()
                    );
                    write_version(root.path(), &Version::parse("0.6.1-nightly").unwrap()).unwrap();
                    refresh_lockfile(root.path()).unwrap();
                    let lock: DocumentMut = std::fs::read_to_string(root.path().join("Cargo.lock"))
                        .unwrap()
                        .parse()
                        .unwrap();
                    let packages = lock["package"].as_array_of_tables().unwrap();
                    assert_eq!(packages.len(), 3);
                    assert!(
                        packages
                            .iter()
                            .all(|package| package["version"].as_str() == Some("0.6.1-nightly"))
                    );
                    let workspace = crate::core::model::Workspace::new(root.path().to_owned());
                    assert_eq!(
                        read_version(&workspace).unwrap(),
                        Version::parse("0.6.1-nightly").unwrap()
                    );
                    assert!(
                        Command::new(&cargo)
                            .args(["metadata", "--locked", "--offline", "--format-version", "1"])
                            .stdout(std::process::Stdio::null())
                            .current_dir(root.path())
                            .status()
                            .unwrap()
                            .success()
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "write_version" {
            case "Updates only workspace version and keeps comments and dependency versions" {
                verify "update the version field without rewriting comments or dependency versions" {
                    let root = tempfile::tempdir().unwrap();
                    let path = root.path().join("Cargo.toml");
                    std::fs::write(
                                &path,
                                "# Keep this comment\n[workspace.package]\nversion = '0.6.0'\nedition = '2024'\n[workspace.dependencies]\nserde = { version = '1', features = ['derive'] }\n",
                            )
                            .unwrap();
                    write_version(root.path(), &Version::parse("1.2.3-rc.1").unwrap()).unwrap();
                    let manifest = std::fs::read_to_string(path).unwrap();
                    assert!(manifest.contains("# Keep this comment"));
                    assert!(manifest.contains("version = \"1.2.3-rc.1\""));
                    assert!(manifest.contains("edition = '2024'"));
                    assert!(manifest.contains("version = '1'"));
                }
            }
        }
    }
}
