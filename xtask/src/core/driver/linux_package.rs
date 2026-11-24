use super::package_files::{copy_dir_recursive, copy_first_rpm, set_executable};
use super::process::run_command;
use super::workspace_manifest::read_version;
use crate::core::model::{DebVersion, RpmVersion, Workspace};
use anyhow::Result;
use std::path::Path;
use std::process::Command;

pub(crate) fn build_deb(workspace: &Workspace, binary: &Path, output_dir: &Path) -> Result<()> {
    let semver = read_version(workspace)?;
    let version = DebVersion::from(&semver);
    let packaging = workspace.root().join("packaging/linux");
    let work_dir = tempfile::tempdir_in("/tmp")?;
    let root = work_dir.path().join("debian/keyringd");
    copy_dir_recursive(&packaging.join("rootfs"), &root)?;
    set_executable(&root.join("usr/libexec/keyringd/grant-tpm-access"))?;
    std::fs::create_dir_all(root.join("usr/bin"))?;
    std::fs::copy(binary, root.join("usr/bin/keyringd"))?;
    set_executable(&root.join("usr/bin/keyringd"))?;
    run_command(
        Command::new("strip")
            .arg("--strip-unneeded")
            .arg(root.join("usr/bin/keyringd")),
    )?;

    super::deb_package::build(
        workspace,
        &packaging,
        work_dir.path(),
        "keyringd",
        &version,
        Some(Path::new("usr/bin/keyringd")),
        &output_dir.join(format!("keyringd_{version}_amd64.deb")),
    )
}

pub(crate) fn build_rpm(workspace: &Workspace, binary: &Path, output_dir: &Path) -> Result<()> {
    let semver = read_version(workspace)?;
    let rpm_ver = RpmVersion::from(&semver);
    let packaging = workspace.root().join("packaging/linux");
    let work_dir = tempfile::tempdir_in("/tmp")?;
    let top_dir = work_dir.path().join("rpmbuild");

    for sub in ["BUILD", "BUILDROOT", "RPMS", "SOURCES", "SPECS", "SRPMS"] {
        std::fs::create_dir_all(top_dir.join(sub))?;
    }
    let sources = top_dir.join("SOURCES");
    std::fs::copy(binary, sources.join("keyringd"))?;
    set_executable(&sources.join("keyringd"))?;
    copy_dir_recursive(&packaging.join("rootfs"), &sources.join("rootfs"))?;
    std::fs::copy(workspace.root().join("LICENSE"), sources.join("LICENSE"))?;

    run_command(
        Command::new("rpmbuild")
            .arg("--define")
            .arg(format!("_topdir {}", top_dir.display()))
            .arg("--define")
            .arg("_userunitdir /usr/lib/systemd/user")
            .arg("--define")
            .arg("_userpresetdir /usr/lib/systemd/user-preset")
            .arg("--define")
            .arg(format!("pkg_version {}", rpm_ver.version()))
            .arg("--define")
            .arg(format!("pkg_release {}", rpm_ver.release()))
            .arg("-bb")
            .arg(packaging.join("rpm/keyringd.spec")),
    )?;

    copy_first_rpm(&top_dir.join("RPMS"), output_dir)
}
