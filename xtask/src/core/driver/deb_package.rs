use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, ensure};

use super::{package_files::set_executable, process::run_command};
use crate::core::model::{DebVersion, Workspace};

pub(super) fn build(
    workspace: &Workspace,
    packaging: &Path,
    staging: &Path,
    name: &str,
    version: &DebVersion,
    shared_library_binary: Option<&Path>,
    output: &Path,
) -> Result<()> {
    let root = staging.join("debian").join(name);
    let metadata = root.join("DEBIAN");
    std::fs::create_dir_all(&metadata)?;

    let dependencies = if let Some(binary) = shared_library_binary {
        shared_library_dependencies(staging, name, &root.join(binary))?
    } else {
        String::new()
    };
    let dependency_prefix = if dependencies.is_empty() {
        String::new()
    } else {
        format!("{dependencies}, ")
    };
    let control = std::fs::read_to_string(packaging.join("deb/control"))?
        .replace("\r\n", "\n")
        .replace("@PACKAGE_VERSION@", version.as_str())
        .replace("@SHLIB_DEPENDENCIES@", &dependency_prefix);
    std::fs::write(metadata.join("control"), control)?;
    for script in ["postinst", "postrm", "prerm"] {
        let contents =
            std::fs::read_to_string(packaging.join("deb").join(format!("{name}.{script}")))?;
        let path = metadata.join(script);
        std::fs::write(&path, contents.replace("\r\n", "\n"))?;
        set_executable(&path)?;
    }
    let documentation = root.join("usr/share/doc").join(name);
    std::fs::create_dir_all(&documentation)?;
    std::fs::copy(
        workspace.root().join("LICENSE"),
        documentation.join("copyright"),
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(
            documentation.join("copyright"),
            std::fs::Permissions::from_mode(0o644),
        )?;
    }

    run_command(
        Command::new("dpkg-deb")
            .args(["--root-owner-group", "--build"])
            .arg(&root)
            .arg(output),
    )?;
    println!("{}", output.display());
    Ok(())
}

fn shared_library_dependencies(staging: &Path, name: &str, binary: &Path) -> Result<String> {
    // dpkg-shlibdeps needs package names, but not a changelog or source build.
    std::fs::write(
        staging.join("debian/control"),
        format!("Source: {name}\n\nPackage: {name}\nArchitecture: amd64\n"),
    )?;
    let output = Command::new("dpkg-shlibdeps")
        .arg("-O")
        .arg("-e")
        .arg(binary)
        .current_dir(staging)
        .output()
        .context("failed to run dpkg-shlibdeps")?;
    ensure!(
        output.status.success(),
        "dpkg-shlibdeps failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = String::from_utf8(output.stdout)?;
    output
        .lines()
        .find_map(|line| line.strip_prefix("shlibs:Depends="))
        .map(str::to_owned)
        .context("dpkg-shlibdeps did not report shared library dependencies")
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    keyringd_testkit::scenario! {
        group "build" {
            case "Packages preserve metadata payload and install scripts without changelogs" {
                verify "retain package metadata, payload, and install scripts while omitting changelogs" {
                    let workspace = Workspace::new(
                        Path::new(env!("CARGO_MANIFEST_DIR"))
                            .parent()
                            .unwrap()
                            .to_owned(),
                    );
                    let output = tempfile::tempdir().unwrap();
                    let runtime = tempfile::tempdir().unwrap();
                    std::fs::write(runtime.path().join("keyringd.exe"), b"windows companion").unwrap();
                    std::fs::write(runtime.path().join("runtime.dll"), b"runtime dependency").unwrap();
                    super::super::linux_package::build_deb(&workspace, Path::new("/bin/true"), output.path())
                        .unwrap();
                    super::super::wsl_package::build_deb(
                        &workspace,
                        Path::new("/bin/true"),
                        &runtime.path().join("keyringd.exe"),
                        runtime.path(),
                        output.path(),
                    )
                    .unwrap();

                    let version =
                        DebVersion::from(&super::super::workspace_manifest::read_version(&workspace).unwrap());
                    for (name, executable, socket) in [
                        ("keyringd", "keyringd", "keyringd-agent.socket"),
                        (
                            "keyringd-wsl",
                            "keyringd-wsl-proxy",
                            "keyringd.socket",
                        ),
                    ] {
                        let package = output.path().join(format!("{name}_{version}_amd64.deb"));
                        let metadata = Command::new("dpkg-deb")
                            .arg("--field")
                            .arg(&package)
                            .output()
                            .unwrap();
                        assert!(metadata.status.success());
                        let metadata = String::from_utf8(metadata.stdout).unwrap();
                        assert!(metadata.contains(&format!("Package: {name}\n")));
                        assert!(metadata.contains(&format!("Version: {version}\n")));
                        assert!(metadata.contains("Maintainer: Keyringd maintainers <info@keyringd.com>"));
                        assert!(metadata.contains("init-system-helpers (>= 1.61)"));
                        assert!(!metadata.contains("@PACKAGE_VERSION@"));
                        assert!(!metadata.contains("@SHLIB_DEPENDENCIES@"));
                        if name == "keyringd" {
                            assert!(metadata.contains("libc6 (>= "));
                        }

                        let extracted = tempfile::tempdir().unwrap();
                        assert!(
                            Command::new("dpkg-deb")
                                .arg("--raw-extract")
                                .arg(&package)
                                .arg(extracted.path())
                                .status()
                                .unwrap()
                                .success()
                        );
                        assert!(extracted.path().join("usr/bin").join(executable).is_file());
                        assert!(
                            extracted
                                .path()
                                .join("usr/lib/systemd/user")
                                .join(socket)
                                .is_file()
                        );
                        assert!(
                            extracted
                                .path()
                                .join("usr/share/doc")
                                .join(name)
                                .join("copyright")
                                .is_file()
                        );
                        assert!(
                            !extracted
                                .path()
                                .join("usr/share/doc")
                                .join(name)
                                .join("changelog.Debian.gz")
                                .exists()
                        );
                        for script in ["postinst", "postrm", "prerm"] {
                            let path = extracted.path().join("DEBIAN").join(script);
                            let contents = std::fs::read_to_string(&path).unwrap();
                            assert!(!contents.contains("#DEBHELPER#"));
                            assert!(!contents.contains('\r'));
                            assert_eq!(
                                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                                0o755
                            );
                            assert!(
                                Command::new("sh")
                                    .arg("-n")
                                    .arg(path)
                                    .status()
                                    .unwrap()
                                    .success()
                            );
                        }
                        if name == "keyringd-wsl" {
                            assert_eq!(
                                std::fs::read(
                                    extracted
                                        .path()
                                        .join("usr/libexec/keyringd/keyringd.exe")
                                )
                                .unwrap(),
                                b"windows companion"
                            );
                            assert_eq!(
                                std::fs::read(extracted.path().join("usr/libexec/keyringd/runtime.dll"))
                                    .unwrap(),
                                b"runtime dependency"
                            );
                        }
                    }
                }
            }
        }
    }
}
