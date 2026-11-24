use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
const ENVIRONMENT_FILE: &str = "environment.d/60-keyringd.conf";
const MANAGED_FILES: [(&str, &str); 3] = [
    (
        ENVIRONMENT_FILE,
        "SSH_AUTH_SOCK=${XDG_RUNTIME_DIR}/keyringd-agent.sock\n",
    ),
    (
        "systemd/user/gcr-ssh-agent.socket.d/60-keyringd.conf",
        "[Socket]\nExecStartPost=\n",
    ),
    (
        "systemd/user/ssh-agent.socket.d/60-keyringd.conf",
        "[Socket]\nExecStartPost=\nExecStopPre=\n",
    ),
];

pub(crate) fn has_managed_agent_configuration(config_directory: &Path) -> std::io::Result<bool> {
    let mut complete = true;
    for (file, contents) in MANAGED_FILES {
        complete &= check_managed_file(&config_directory.join(file), contents)?;
    }
    Ok(complete)
}

pub(crate) fn configure(config_directory: &Path) -> std::io::Result<()> {
    let mut missing = Vec::new();
    for (file, contents) in MANAGED_FILES {
        let path = config_directory.join(file);
        if !check_managed_file(&path, contents)? {
            missing.push((path, contents));
        }
    }
    ensure_private_directory(config_directory)?;
    ensure_private_directory(&config_directory.join("environment.d"))?;
    for (path, _) in &missing {
        std::fs::create_dir_all(
            path.parent()
                .expect("managed files have a parent directory"),
        )?;
    }
    for (index, (path, contents)) in missing.iter().enumerate() {
        if let Err(error) = write_atomic(path, contents.as_bytes(), 0o600) {
            for (created, _) in missing[..index].iter().rev() {
                std::fs::remove_file(created)?;
            }
            return Err(error);
        }
    }
    Ok(())
}

pub(crate) fn unconfigure(config_directory: &Path) -> std::io::Result<()> {
    let mut managed = Vec::new();
    for (file, contents) in MANAGED_FILES {
        let path = config_directory.join(file);
        if check_managed_file(&path, contents)? {
            managed.push(path);
        }
    }
    for path in managed.into_iter().rev() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn read_configuration(path: &Path) -> std::io::Result<Option<String>> {
    reject_non_regular_file(path)?;
    match std::fs::read_to_string(path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn check_managed_file(path: &Path, expected: &str) -> std::io::Result<bool> {
    match read_configuration(path)? {
        None => Ok(false),
        Some(contents) if contents == expected => Ok(true),
        Some(_) => Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "{} contains settings not managed by Keyringd.",
                path.display()
            ),
        )),
    }
}
fn ensure_private_directory(path: &Path) -> std::io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
            return Ok(());
        }
        Ok(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("{} is not a regular directory", path.display()),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir(path)?;
        }
        Err(error) => return Err(error),
    }
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
}

fn reject_non_regular_file(path: &Path) -> std::io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {
            Ok(())
        }
        Ok(_) => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{} is not a regular file", path.display()),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn write_atomic(path: &Path, contents: &[u8], mode: u32) -> std::io::Result<()> {
    reject_non_regular_file(path)?;
    let temporary = temporary_path(path);
    let mut created = false;
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(mode)
            .open(&temporary)?;
        created = true;
        file.set_permissions(std::fs::Permissions::from_mode(mode))?;
        file.write_all(contents)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)
    })();
    if created && result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn temporary_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("config");
    path.with_file_name(format!(".{file_name}.{}.tmp", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "configure" {
            case "Publishes a discoverable per-user Agent endpoint to OpenSSH" {
                spec "https://www.rfc-editor.org/rfc/rfc9987.html#section-6"
                given "a private configuration directory and runtime directory" {
                    let directory = tempfile::tempdir().unwrap();
                    let config = directory.path().join("config");
                    let runtime = directory.path().join("runtime");
                    std::fs::create_dir(&runtime).unwrap();
                    configure(&config).unwrap();
                }
                when "systemd generates the user environment from the installed configuration" {
                    let output = std::process::Command::new(
                        "/usr/lib/systemd/user-environment-generators/30-systemd-environment-d-generator",
                    )
                    .env("XDG_CONFIG_HOME", &config)
                    .env("XDG_RUNTIME_DIR", &runtime)
                    .env("HOME", directory.path())
                    .env_remove("SSH_AUTH_SOCK")
                    .output()
                    .unwrap();
                }
                then "SSH_AUTH_SOCK points OpenSSH to the per-user Agent socket" {
                    assert!(
                        output.status.success(),
                        "{}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                    let expected = format!("SSH_AUTH_SOCK={}/keyringd-agent.sock", runtime.display());
                    assert!(
                        String::from_utf8_lossy(&output.stdout)
                            .lines()
                            .any(|line| line == expected),
                        "{}",
                        String::from_utf8_lossy(&output.stdout)
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "configure" {
            case "Configure and unconfigure preserve other settings" {
                verify "retain unrelated configuration entries when toggling the agent setting" {
                    let home = tempfile::tempdir().unwrap();
                    std::fs::create_dir_all(home.path().join(".config/environment.d")).unwrap();
                    let other = home.path().join(".config/environment.d/20-other.conf");
                    std::fs::write(&other, "OTHER=value\n").unwrap();
                    configure(&home.path().join(".config")).unwrap();
                    configure(&home.path().join(".config")).unwrap();
                    assert!(has_managed_agent_configuration(&home.path().join(".config")).unwrap());
                    unconfigure(&home.path().join(".config")).unwrap();
                    unconfigure(&home.path().join(".config")).unwrap();
                    assert!(!has_managed_agent_configuration(&home.path().join(".config")).unwrap());
                    assert_eq!(std::fs::read_to_string(other).unwrap(), "OTHER=value\n");
                    assert!(!home.path().join(".ssh").exists());
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "unconfigure" {
            case "Changed file is preserved" {
                verify "retain the externally modified SSH configuration file during removal" {
                    let home = tempfile::tempdir().unwrap();
                    configure(&home.path().join(".config")).unwrap();
                    std::fs::write(home.path().join(".config").join(ENVIRONMENT_FILE), "SSH_AUTH_SOCK=other\n").unwrap();
                    assert!(configure(&home.path().join(".config")).is_err());
                    assert!(unconfigure(&home.path().join(".config")).is_err());
                    assert_eq!(
                        std::fs::read_to_string(home.path().join(".config").join(ENVIRONMENT_FILE)).unwrap(),
                        "SSH_AUTH_SOCK=other\n"
                    );
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "unconfigure" {
            case "Symlink is preserved" {
                verify "preserve a symlink encountered at the SSH configuration path" {
                    let home = tempfile::tempdir().unwrap();
                    std::fs::create_dir_all(home.path().join(".config/environment.d")).unwrap();
                    let target = home.path().join("other");
                    std::fs::write(&target, "other").unwrap();
                    std::os::unix::fs::symlink(&target, home.path().join(".config").join(ENVIRONMENT_FILE)).unwrap();
                    assert!(configure(&home.path().join(".config")).is_err());
                    assert!(unconfigure(&home.path().join(".config")).is_err());
                    assert_eq!(std::fs::read_to_string(target).unwrap(), "other");
                }
            }
        }
    }
}
