use crate::core::{
    driver::{cargo, workspace_manifest},
    model::Workspace,
};
use anyhow::{Context, Result};
use semver::Version;

pub(crate) fn set(workspace: &Workspace, input: &str) -> Result<()> {
    let version = Version::parse(input).context("invalid release version")?;
    workspace_manifest::write_version(workspace.root(), &version)?;
    cargo::refresh_lockfile(workspace.root())
}

pub(crate) fn bump(workspace: &Workspace) -> Result<Version> {
    let mut version = workspace_manifest::read_version(workspace)?;
    version.patch = version
        .patch
        .checked_add(1)
        .context("patch version overflow")?;
    version.pre = semver::Prerelease::EMPTY;
    version.build = semver::BuildMetadata::EMPTY;
    set(workspace, &version.to_string())?;
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "version::bump" {
            case "Patch bumps update inherited package versions in both manifest and lockfile" {
                verify "stable and suffixed releases advance to the next development patch" {
                    for current in ["0.6.3", "0.6.3-rc.1+build.7"] {
                        let directory = tempfile::tempdir().unwrap();
                        let root = directory.path();
                        std::fs::write(root.join("Cargo.toml"), format!(
                            "[workspace]\nresolver = '2'\nmembers = ['one', 'two']\n[workspace.package]\nversion = '{current}'\nedition = '2024'\n"
                        )).unwrap();
                        for name in ["one", "two"] {
                            let package = root.join(name);
                            std::fs::create_dir_all(package.join("src")).unwrap();
                            std::fs::write(package.join("src/lib.rs"), "").unwrap();
                            std::fs::write(package.join("Cargo.toml"), format!(
                                "[package]\nname = '{name}'\nversion.workspace = true\nedition.workspace = true\n"
                            )).unwrap();
                        }
                        let workspace = Workspace::new(root.to_owned());
                        assert_eq!(bump(&workspace).unwrap(), Version::parse("0.6.4").unwrap());
                        assert_eq!(workspace_manifest::read_version(&workspace).unwrap().to_string(), "0.6.4");
                        let lock: toml_edit::DocumentMut = std::fs::read_to_string(root.join("Cargo.lock")).unwrap().parse().unwrap();
                        let packages = lock["package"].as_array_of_tables().unwrap();
                        assert_eq!(packages.len(), 2);
                        assert!(packages.iter().all(|package| package["version"].as_str() == Some("0.6.4")));
                    }
                }
            }
        }
    }
}
