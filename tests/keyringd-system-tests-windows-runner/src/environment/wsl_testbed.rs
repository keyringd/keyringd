use anyhow::{Context, Result, ensure};
use keyringd_system_tests::environment::windows::command_output::run_command;
use keyringd_wsl_system_tests::WslPackageInstallation;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use windows::Win32::Foundation::ERROR_NO_MORE_ITEMS;
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, RRF_RT_REG_SZ, RegCloseKey, RegEnumKeyExW, RegGetValueW,
    RegOpenKeyExW,
};
use windows::core::{PCWSTR, PWSTR, w};

/// Owns only the disposable distribution created for this run.
pub(crate) struct WslTestbed {
    installation: WslPackageInstallation,
    directory: PathBuf,
    registered: bool,
}

impl WslTestbed {
    pub(crate) fn prepare(workspace: &Path, distribution: &str) -> Result<Self> {
        let installation = WslPackageInstallation::new(distribution)?;
        let name = installation.distribution();
        let registered = Self::wsl(&["--list", "--quiet"], Duration::from_secs(60))?;
        ensure!(
            !registered.lines().any(|line| line.trim() == name),
            "Test distribution already exists: {name}; refusing to overwrite it"
        );
        let parent = workspace.join("target/wsl-package-testbeds");
        std::fs::create_dir_all(&parent)?;
        let workspace = workspace.canonicalize()?;
        ensure!(
            parent.canonicalize()?.starts_with(&workspace),
            "WSL testbed directory is outside the workspace"
        );
        let directory = std::path::absolute(&parent)?.join(name);
        let source = if name == "KeyringdE2EUbuntu" {
            "Ubuntu-26.04"
        } else {
            "FedoraLinux-44"
        };
        let mut testbed = Self {
            installation,
            directory,
            registered: false,
        };
        println!(
            "Preparing {source} as {}",
            testbed.installation.distribution()
        );
        Self::wsl(
            &[
                "--install",
                "--distribution",
                source,
                "--name",
                testbed.installation.distribution(),
                "--location",
                testbed
                    .directory
                    .to_str()
                    .context("WSL testbed path must be Unicode")?,
                "--web-download",
                "--no-launch",
                "--version",
                "2",
            ],
            Duration::from_secs(1200),
        )?;
        testbed.registered = true;
        // Retain the resolved path from creation rather than following a changed
        // junction when deciding which registration may be removed later.
        testbed.directory = testbed.directory.canonicalize()?;
        ensure!(
            testbed.directory.starts_with(&workspace),
            "WSL testbed was created outside the workspace"
        );
        testbed.exec(&["touch", "/etc/keyringd-e2e-disposable"])?;
        Self::wsl(
            &[
                "--manage",
                testbed.installation.distribution(),
                "--set-default-user",
                "root",
            ],
            Duration::from_secs(60),
        )?;
        // WSL shares the kernel cgroup hierarchy across distributions. Preserve
        // the existing isolation from a developer's root user manager.
        let slice = if source == "Ubuntu-26.04" {
            "keyringd-e2eubuntu.slice"
        } else {
            "keyringd-e2efedora.slice"
        };
        testbed.exec(&[
            "bash", "-c",
            r#"set -e; mkdir -p /etc/systemd/system/user@0.service.d; printf '[Service]\nSlice=%s\n' "$1" > /etc/systemd/system/user@0.service.d/20-e2e-slice.conf; systemctl daemon-reload"#,
            "isolate-test-manager", slice,
        ])?;
        if source == "Ubuntu-26.04" {
            testbed.exec(&[
                "bash", "-c",
                "set -e; printf '#!/bin/sh\nexit 101\n' > /usr/sbin/policy-rc.d; chmod 0755 /usr/sbin/policy-rc.d; apt-get update; DEBIAN_FRONTEND=noninteractive apt-get install -y ca-certificates openssh-client openssh-server python3",
            ])?;
        } else {
            testbed.exec(&[
                "dnf",
                "install",
                "-y",
                "systemd",
                "openssh-clients",
                "openssh-server",
                "python3",
            ])?;
            // Fedora's empty binfmt.d skips WSL's interpreter registration after
            // a fresh binfmt mount. This is normal WSL /init interoperability.
            testbed.exec(&[
                "bash", "-c",
                "set -e; mkdir -p /etc/binfmt.d; printf ':WSLInterop:M::MZ::/init:P\n' > /etc/binfmt.d/WSLInterop.conf; systemctl restart systemd-binfmt.service",
            ])?;
        }
        testbed.installation.terminate()?;
        Ok(testbed)
    }

    pub(crate) fn installation(&self) -> &WslPackageInstallation {
        &self.installation
    }

    pub(crate) fn close(mut self) -> Result<()> {
        let result = self.remove();
        self.registered = false;
        result
    }

    fn remove(&self) -> Result<()> {
        // Verify both the Windows registration and the Linux marker before
        // unregistering: a familiar name alone does not establish ownership.
        let actual = self.registration_directory()?.canonicalize()?;
        let expected = &self.directory;
        ensure!(
            actual == *expected,
            "Refusing to remove {} outside {} (registered at {})",
            self.installation.distribution(),
            expected.display(),
            actual.display()
        );
        self.exec(&["test", "-f", "/etc/keyringd-e2e-disposable"])?;
        Self::wsl(
            &["--unregister", self.installation.distribution()],
            Duration::from_secs(180),
        )?;
        Ok(())
    }

    fn exec(&self, arguments: &[&str]) -> Result<String> {
        let mut command = Command::new("wsl.exe");
        command.args([
            "--distribution",
            self.installation.distribution(),
            "--user",
            "root",
            "--exec",
        ]);
        command.args(arguments);
        Self::checked(command, Duration::from_secs(600))
    }

    fn wsl(arguments: &[&str], timeout: Duration) -> Result<String> {
        let mut command = Command::new("wsl.exe");
        command.args(arguments);
        Self::checked(command, timeout)
    }

    fn checked(command: Command, timeout: Duration) -> Result<String> {
        let description = format!("{command:?}");
        println!("WSL testbed: {description}");
        let output = run_command(command, timeout).with_context(|| description.clone())?;
        let stdout = output_text(&output.stdout);
        let stderr = output_text(&output.stderr);
        ensure!(
            output.status.success(),
            "{description} failed: {stdout}\n{stderr}"
        );
        Ok(stdout)
    }

    fn registration_directory(&self) -> Result<PathBuf> {
        let mut key = HKEY::default();
        unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                w!("Software\\Microsoft\\Windows\\CurrentVersion\\Lxss"),
                None,
                KEY_READ,
                &mut key,
            )
            .ok()?
        };
        let result = (|| {
            let mut found = None;
            for index in 0.. {
                let mut name = [0u16; 256];
                let mut length = name.len() as u32;
                let status = unsafe {
                    RegEnumKeyExW(
                        key,
                        index,
                        Some(PWSTR(name.as_mut_ptr())),
                        &mut length,
                        None,
                        None,
                        None,
                        None,
                    )
                };
                if status == ERROR_NO_MORE_ITEMS {
                    break;
                }
                status.ok()?;
                let path = format!(
                    "Software\\Microsoft\\Windows\\CurrentVersion\\Lxss\\{}",
                    String::from_utf16(&name[..length as usize])?
                );
                if registry_string(&path, w!("DistributionName"))?
                    == self.installation.distribution()
                {
                    ensure!(found.is_none(), "Duplicate WSL testbed registration");
                    found = Some(PathBuf::from(registry_string(&path, w!("BasePath"))?));
                }
            }
            found.context("Could not find the disposable WSL registration")
        })();
        unsafe {
            let _ = RegCloseKey(key);
        }
        result
    }
}

impl Drop for WslTestbed {
    fn drop(&mut self) {
        if self.registered
            && let Err(error) = self.remove()
        {
            eprintln!("Could not clean up WSL testbed: {error:#}");
        }
    }
}

fn registry_string(path: &str, value: PCWSTR) -> Result<String> {
    let path = path.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    let mut size = 0;
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(path.as_ptr()),
            value,
            RRF_RT_REG_SZ,
            None,
            None,
            Some(&mut size),
        )
        .ok()?;
    }
    let mut buffer = vec![0u16; size as usize / 2];
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(path.as_ptr()),
            value,
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
        .ok()?;
    }
    Ok(String::from_utf16(&buffer[..size as usize / 2])?
        .trim_end_matches('\0')
        .to_owned())
}

fn output_text(bytes: &[u8]) -> String {
    // wsl.exe host commands emit UTF-16LE when redirected; Linux commands use UTF-8.
    if bytes.contains(&0) {
        let wide = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        String::from_utf16_lossy(&wide)
            .trim_start_matches('\u{feff}')
            .to_owned()
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}
