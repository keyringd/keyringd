use anyhow::{Context, Result, ensure};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

pub struct WslPackageInstallation {
    distribution: &'static str,
}

impl WslPackageInstallation {
    pub fn new(distribution: &str) -> Result<Self> {
        let distribution = match distribution {
            "ubuntu" | "KeyringdE2EUbuntu" => "KeyringdE2EUbuntu",
            "fedora" | "KeyringdE2EFedora" => "KeyringdE2EFedora",
            _ => anyhow::bail!("WSL system tests require a managed disposable distribution"),
        };
        Ok(Self { distribution })
    }

    pub fn from_environment() -> Result<Self> {
        Self::new(&std::env::var("KEYRINGD_TEST_WSL_DISTRO")?)
    }

    pub fn distribution(&self) -> &str {
        self.distribution
    }

    pub fn terminate(&self) -> Result<()> {
        self.checked(&["test", "-f", "/etc/keyringd-e2e-disposable"])?;
        let output = Command::new("wsl.exe")
            .args(["--terminate", self.distribution])
            .output()?;
        ensure!(
            output.status.success(),
            "could not stop disposable WSL distribution: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(())
    }

    pub fn companion_path(&self) -> Result<PathBuf> {
        self.checked(&["test", "-f", "/etc/keyringd-e2e-disposable"])?;
        Ok(PathBuf::from(
            self.checked(&["wslpath", "-w", "/usr/libexec/keyringd/keyringd.exe"])?
                .trim(),
        ))
    }

    fn run(&self, arguments: &[&str]) -> Result<std::process::Output> {
        let mut command = Command::new("wsl.exe");
        command
            .args([
                "--distribution",
                self.distribution,
                "--user",
                "root",
                "--exec",
            ])
            .args(arguments);
        Ok(
            keyringd_system_tests::environment::windows::command_output::run_command(
                command,
                Duration::from_secs(180),
            )?,
        )
    }

    fn checked(&self, arguments: &[&str]) -> Result<String> {
        let output = self.run(arguments)?;
        ensure!(
            output.status.success(),
            "{} {arguments:?} failed: {}\n{}",
            self.distribution,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(String::from_utf8(output.stdout)?)
    }

    pub fn install(&self, directory: &std::path::Path) -> Result<String> {
        let directory = std::path::absolute(directory)?;
        let (extension, package_manager) = if self.distribution == "KeyringdE2EUbuntu" {
            ("deb", "apt-get")
        } else {
            ("rpm", "dnf")
        };
        let kernel = self.checked(&["uname", "-r"])?;
        ensure!(
            kernel.to_lowercase().contains("wsl2"),
            "package tests require WSL2: {kernel}"
        );
        self.checked(&["test", "-f", "/etc/keyringd-e2e-disposable"])
            .context("Package installation requires a disposable distribution managed by the Windows runner")?;
        self.checked(&["test", "-f", "/proc/sys/fs/binfmt_misc/WSLInterop"])
            .context("WSL Windows interoperability must be enabled before package tests")?;
        let packages = std::fs::read_dir(&directory)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        let packages = packages
            .iter()
            .filter(|path| path.extension().is_some_and(|value| value == extension))
            .collect::<Vec<_>>();
        ensure!(
            packages.len() == 1,
            "expected exactly one .{extension} package in {}",
            directory.display()
        );
        let path = packages[0]
            .to_str()
            .context("package path must be Unicode")?;
        let path = self.checked(&["wslpath", "-a", path])?;
        if extension == "deb" {
            self.checked(&[package_manager, "install", "--reinstall", "-y", path.trim()])
        } else {
            let installed = self.run(&["rpm", "-q", "keyringd-wsl"])?;
            let action = if installed.status.success() {
                "reinstall"
            } else {
                "install"
            };
            self.checked(&[package_manager, action, "-y", path.trim()])
        }
    }

    pub fn verify(&self) -> Result<String> {
        for resource in [
            "/usr/libexec/keyringd/Microsoft.UI.Xaml.Resources.19H1.dll",
            "/usr/libexec/keyringd/Microsoft.UI.Xaml.resources.common.dll",
            "/usr/libexec/keyringd/en-US/Microsoft.ui.xaml.dll.mui",
        ] {
            self.checked(&["test", "-f", resource])?;
        }
        self.checked(&["test", "-x", "/usr/bin/keyringd-wsl-proxy"])?;
        self.checked(&["test", "-x", "/usr/libexec/keyringd/keyringd.exe"])?;
        let units = self.checked(&[
            "systemd-analyze",
            "verify",
            "/usr/lib/systemd/user/keyringd.socket",
            "/usr/lib/systemd/user/keyringd.service",
        ])?;
        let enabled = self.checked(&["systemctl", "--global", "is-enabled", "keyringd.socket"])?;
        let output = self.run(&["bash", "-c", "set -e; directory=$(mktemp -d /root/keyringd-package.XXXXXXXX); trap 'rm -rf -- \"$directory\"' EXIT; set +e; XDG_RUNTIME_DIR=\"$directory\" timeout 2 /usr/bin/keyringd-wsl-proxy --windows-exe /usr/libexec/keyringd/keyringd.exe; status=$?; exit \"$status\""])?;
        ensure!(
            output.status.code() == Some(124),
            "installed proxy did not remain running: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(format!(
            "{units}\nSocket: {enabled}\nProxy remained running until the test timeout."
        ))
    }
}
