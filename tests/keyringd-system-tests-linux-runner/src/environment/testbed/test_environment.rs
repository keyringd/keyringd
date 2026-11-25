use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};

use super::ContainerCommand;
use super::SystemContainer;
use crate::Distribution;
use crate::environment::testbed::distribution::{
    matches_package, package_build_command, package_install_command,
};
use crate::environment::testbed::virtual_bluetooth::{VirtualBluetooth, controllers};

const APPLICATION_ENVIRONMENT: &[&str] = &[
    "DISPLAY=:90",
    "GDK_BACKEND=x11",
    "GTK_A11Y=atspi",
    "GSK_RENDERER=cairo",
    "XDG_CONFIG_HOME=/var/lib/keyringd-e2e/config",
    "XDG_DATA_HOME=/var/lib/keyringd-e2e/data",
    "XDG_CACHE_HOME=/var/lib/keyringd-e2e/cache",
    "TPM2TOOLS_TCTI=swtpm:host=127.0.0.1,port=2321",
    "RUST_LOG=keyringd_fido_client=debug,keyringd=info",
];

pub(super) struct TestEnvironment<'a> {
    command: ContainerCommand<'a>,
    distribution: Distribution,
    workspace: &'a Path,
    package: Option<&'a Path>,
    test_executable: &'a Path,
    usb_control_socket: &'a Path,
    report_directory: &'a Path,
    authenticator_control_url: &'a str,
    filter: Option<&'a str>,
    bluetooth: Option<&'a VirtualBluetooth>,
    interrupted: &'a AtomicBool,
}

impl<'a> TestEnvironment<'a> {
    pub(super) fn new(
        container: &'a SystemContainer,
        bluetooth: Option<&'a VirtualBluetooth>,
        interrupted: &'a AtomicBool,
    ) -> Self {
        Self {
            command: container.command(),
            distribution: container.distribution,
            workspace: &container.workspace,
            package: container.package.as_deref(),
            test_executable: &container.test_executable,
            usb_control_socket: &container.usb_control_socket,
            report_directory: &container.report_directory,
            authenticator_control_url: &container.authenticator_control_url,
            filter: container.filter.as_deref(),
            bluetooth,
            interrupted,
        }
    }

    pub(super) fn execute(self) -> Result<()> {
        let package = self.prepare_package()?;
        self.install_application(&package)?;
        self.configure_hybrid_tunnel()?;
        self.prepare_directories()?;
        self.start_services()?;
        self.configure_bluetooth_adapter_visibility()?;
        self.configure_bluetooth()?;
        self.configure_user_manager()?;
        self.record_execution_environment()?;
        self.run_tests()
    }

    fn record_execution_environment(&self) -> Result<()> {
        let report = self
            .workspace
            .join(self.report_directory.strip_prefix("/workspace")?);
        // These are the binaries inside the testbed, not the WSL host's SSH.
        for (name, script) in [
            ("SSH client (testbed)", "command -v ssh && exec ssh -V"),
            (
                "SSH server (testbed)",
                "command -v /usr/sbin/sshd && exec /usr/sbin/sshd -V",
            ),
        ] {
            let observed = self.command.output_as_e2e_with_options(
                "SSH version for report",
                ["--property=RuntimeMaxSec=5s"],
                ["sh", "-c", script],
            );
            let (path, version) = match observed {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let path = stdout.lines().next().map(std::path::PathBuf::from);
                    (
                        path,
                        Ok(String::from_utf8_lossy(&output.stderr).trim().to_owned()),
                    )
                }
                Err(error) => (None, Err(format!("{error:#}"))),
            };
            keyringd_testkit::runner::ReportRunMetadata::record_tool(
                &report,
                keyringd_testkit::runner::ToolExecutionMetadata {
                    name: name.into(),
                    path,
                    version,
                },
            )?;
        }
        let os = self
            .command
            .output_as_e2e_with_options(
                "testbed OS for report",
                ["--property=RuntimeMaxSec=5s"],
                ["cat", "/etc/os-release"],
            )
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
            .map_err(|error| format!("{error:#}"));
        keyringd_testkit::runner::ReportRunMetadata::record_test_environment(&report, os)
    }

    fn configure_bluetooth_adapter_visibility(&self) -> Result<()> {
        self.command.run_as_root(
            "Bluetooth adapter visibility fixture installation",
            [
                "install",
                "-D",
                "--mode=0755",
                "--owner=root",
                "--group=root",
                "/workspace/tests/keyringd-system-tests-linux-runner/testbed/bluetooth-adapters",
                "/usr/local/libexec/keyringd-e2e-bluetooth-adapters",
            ],
        )?;
        self.command.run_as_root(
            "Bluetooth adapter visibility fixture permission",
            [
                "sh", "-c",
                "printf '%s\\n' 'e2e ALL=(root) NOPASSWD: /usr/local/libexec/keyringd-e2e-bluetooth-adapters hide, /usr/local/libexec/keyringd-e2e-bluetooth-adapters show' > /etc/sudoers.d/keyringd-e2e-bluetooth-visibility && chmod 0440 /etc/sudoers.d/keyringd-e2e-bluetooth-visibility && visudo -cf /etc/sudoers.d/keyringd-e2e-bluetooth-visibility",
            ],
        )
    }

    fn configure_bluetooth(&self) -> Result<()> {
        let Some(bluetooth) = self.bluetooth else {
            return Ok(());
        };
        self.command.run_as_root(
            "BlueZ service start",
            ["systemctl", "start", "bluetooth.service"],
        )?;
        let controller = &bluetooth.scanner;
        ensure!(
            controller.strip_prefix("hci").is_some_and(|index| {
                !index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit())
            }),
            "invalid virtual Bluetooth controller name: {controller}"
        );
        for (description, setting) in [
            ("virtual Bluetooth LE enablement", "le"),
            ("virtual Bluetooth controller power on", "power"),
        ] {
            self.command
                .run_as_root_with_timeout(
                    description,
                    Duration::from_secs(15),
                    // btmgmt attaches stdin to its event loop. A pseudo-terminal
                    // lets it dispatch commands through systemd-run --pipe.
                    [
                        "script",
                        "-qefc",
                        &format!("btmgmt -i {controller} {setting} on"),
                        "/dev/null",
                    ],
                )
                .with_context(|| {
                    let visible = controllers()
                        .map(|names| format!("{names:?}"))
                        .unwrap_or_else(|error| format!("unavailable: {error:#}"));
                    format!(
                        "Bluetooth setup failed for {controller}; host-visible controllers: {visible}"
                    )
                })?;
        }
        // Availability scenarios toggle the PC adapter. The Agent still runs
        // without this privilege; Phone advertising uses the HCI socket.
        self.command.run_as_root(
            "virtual Bluetooth availability permission",
            [
                "sh", "-c",
                "printf '%s\\n' 'e2e ALL=(root) NOPASSWD: /usr/bin/btmgmt' > /etc/sudoers.d/keyringd-e2e-bluetooth && chmod 0440 /etc/sudoers.d/keyringd-e2e-bluetooth",
            ],
        )
    }

    fn configure_hybrid_tunnel(&self) -> Result<()> {
        let source = self.report_directory.join("runtime/hybrid-tunnel.crt");
        let destination = match self.distribution {
            Distribution::Ubuntu => "/usr/local/share/ca-certificates/keyringd-e2e-hybrid.crt",
            Distribution::Fedora => "/etc/pki/ca-trust/source/anchors/keyringd-e2e-hybrid.crt",
        };
        self.command.run_as_root(
            "Hybrid tunnel CA installation",
            [
                "install".as_ref(),
                "--mode=0644".as_ref(),
                source.as_os_str(),
                destination.as_ref(),
            ],
        )?;
        let update_command = match self.distribution {
            Distribution::Ubuntu => "update-ca-certificates",
            Distribution::Fedora => "update-ca-trust",
        };
        self.command
            .run_as_root("Hybrid tunnel CA trust update", [update_command])?;
        self.command.run_as_root(
            "Hybrid tunnel name resolution",
            [
                "sh",
                "-c",
                "printf '127.0.0.1 cable.ua5v.com\\n::1 cable.ua5v.com\\n' >> /etc/hosts",
            ],
        )?;
        self.command.run_as_root(
            "Hybrid tunnel loopback resolution verification",
            [
                "bash",
                "-euo",
                "pipefail",
                "-c",
                "getent ahostsv4 cable.ua5v.com | awk '$1 != \"127.0.0.1\" { exit 1 } END { if (NR == 0) exit 1 }'; getent ahostsv6 cable.ua5v.com | awk '$1 != \"::1\" { exit 1 } END { if (NR == 0) exit 1 }'",
            ],
        )
    }

    fn prepare_package(&self) -> Result<PathBuf> {
        if let Some(package) = self.package {
            ensure!(
                matches_package(self.distribution, package),
                "{} is not a {} package",
                package.display(),
                self.distribution
            );
            return Ok(package.to_path_buf());
        }

        let output = self.workspace.join(format!(
            "target/linux-system/packages/{}",
            self.distribution.id()
        ));
        let container_output = Path::new("/workspace").join(
            output
                .strip_prefix(self.workspace)
                .expect("the package output should be inside the workspace because the distribution build writes it there"),
        );
        self.command.run_as_root(
            "package output reset",
            [
                "rm".as_ref(),
                "--recursive".as_ref(),
                "--force".as_ref(),
                container_output.as_os_str(),
            ],
        )?;
        let packages = Path::new("/workspace/target/linux-system/packages");
        self.command.run_as_root(
            "package output preparation",
            [
                "install".as_ref(),
                "--directory".as_ref(),
                "--owner=e2e".as_ref(),
                "--group=e2e".as_ref(),
                packages.as_os_str(),
                container_output.as_os_str(),
            ],
        )?;
        // The testbed is launched by xtask. Reuse that executable for the same
        // dist command as releases, translating its path into the bind mount.
        let xtask = std::env::current_exe().context("failed to locate xtask")?;
        let xtask = Path::new("/workspace").join(
            xtask
                .strip_prefix(self.workspace)
                .context("xtask must be inside the workspace mounted into the testbed")?,
        );
        self.command.run_as_e2e(
            "keyringd package build",
            package_build_command(self.distribution, &xtask, &self.application_binary()?),
        )?;
        let packages = fs::read_dir(&output)?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| matches_package(self.distribution, path))
            .collect::<Vec<_>>();
        ensure!(
            packages.len() == 1,
            "package build produced {} matching files under {}",
            packages.len(),
            output.display()
        );
        let relative = packages[0]
            .strip_prefix(self.workspace)
            .expect("the package output should be inside the workspace because the distribution build writes it there");
        Ok(Path::new("/workspace").join(relative))
    }

    fn application_binary(&self) -> Result<PathBuf> {
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| self.workspace.join("target"));
        Ok(Path::new("/workspace")
            .join(target.strip_prefix(self.workspace)?)
            .join("release/keyringd"))
    }

    fn install_application(&self, package: &Path) -> Result<()> {
        let install = package_install_command(self.distribution, package);
        match self.distribution {
            Distribution::Ubuntu => self.command.run_as_root_with_options(
                "keyringd package installation",
                ["--setenv=DEBIAN_FRONTEND=noninteractive"],
                install,
            ),
            Distribution::Fedora => self
                .command
                .run_as_root("keyringd package installation", install),
        }
    }

    fn prepare_directories(&self) -> Result<()> {
        self.command.run_as_root(
            "E2E temporary directory reset",
            [
                "rm",
                "--recursive",
                "--force",
                "/var/lib/keyringd-e2e",
                "/tmp/keyringd-swtpm",
            ],
        )?;
        self.command.run_as_root(
            "E2E directory preparation",
            [
                "install",
                "--directory",
                "--owner=e2e",
                "--group=e2e",
                "/var/lib/keyringd-e2e",
                "/var/lib/keyringd-e2e/config",
                "/var/lib/keyringd-e2e/data",
                "/var/lib/keyringd-e2e/cache",
                "/tmp/keyringd-swtpm",
            ],
        )
    }

    fn start_services(&self) -> Result<()> {
        self.command.run_as_e2e(
            "software TPM setup",
            [
                "swtpm_setup",
                "--tpm2",
                "--tpm-state",
                "/tmp/keyringd-swtpm",
                "--overwrite",
            ],
        )?;
        self.command.start_e2e_service(
            "Xvfb",
            "keyringd-e2e-xvfb",
            &self.report_directory.join("runtime/xvfb.log"),
            [
                "Xvfb",
                ":90",
                "-screen",
                "0",
                "1280x800x24",
                "-nolisten",
                "tcp",
            ],
        )?;
        self.wait_until("Xvfb", || {
            self.command
                .e2e_command_succeeds(["test", "-S", "/tmp/.X11-unix/X90"])
        })?;
        self.command.start_e2e_service(
            "software TPM",
            "keyringd-e2e-swtpm",
            &self.report_directory.join("runtime/swtpm.log"),
            [
                "swtpm",
                "socket",
                "--tpm2",
                "--flags",
                "not-need-init",
                "--tpmstate",
                "dir=/tmp/keyringd-swtpm",
                "--server",
                "type=tcp,port=2321",
                "--ctrl",
                "type=tcp,port=2322",
            ],
        )?;
        self.wait_until("software TPM", || {
            self.command
                .e2e_command_succeeds(["bash", "-c", "exec 3<>/dev/tcp/127.0.0.1/2322"])
        })?;
        self.command.run_as_e2e(
            "software TPM startup",
            [
                "tpm2_startup",
                "-T",
                "swtpm:host=127.0.0.1,port=2321",
                "--clear",
            ],
        )
    }

    fn configure_user_manager(&self) -> Result<()> {
        for unit in ["keyringd-desktop.service", "keyringd-agent.socket"] {
            let destination = format!("/etc/systemd/user/{unit}.d/e2e-restarts.conf");
            self.command.run_as_root(
                "E2E application restart policy",
                [
                    "install",
                    "-D",
                    "--mode=0644",
                    "/workspace/tests/keyringd-system-tests-linux-runner/testbed/keyringd-e2e-restarts.conf",
                    &destination,
                ],
            )?;
        }
        let mut environment = vec!["systemctl", "--user", "set-environment"];
        environment.extend_from_slice(APPLICATION_ENVIRONMENT);
        self.command
            .run_as_e2e("systemd user environment", environment)?;
        if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
            let profile = PathBuf::from(profile);
            let profile = Path::new("/workspace").join(profile.strip_prefix(self.workspace)?);
            self.command.run_as_e2e(
                "coverage profile environment",
                [
                    "systemctl".to_owned(),
                    "--user".to_owned(),
                    "set-environment".to_owned(),
                    format!("LLVM_PROFILE_FILE={}", profile.display()),
                ],
            )?;
        }
        self.command.run_as_e2e(
            "Wayland environment removal",
            [
                "systemctl",
                "--user",
                "unset-environment",
                "WAYLAND_DISPLAY",
            ],
        )?;
        self.command.run_as_e2e(
            "systemd user daemon reload",
            ["systemctl", "--user", "daemon-reload"],
        )?;
        self.command.run_as_e2e(
            "agent socket activation",
            ["systemctl", "--user", "start", "keyringd-agent.socket"],
        )
    }

    fn run_tests(&self) -> Result<()> {
        ensure!(!self.interrupted.load(Ordering::SeqCst), "interrupted");
        let artifacts = format!(
            "--setenv=KEYRINGD_E2E_ARTIFACTS={}",
            self.report_directory.join("runtime").display()
        );
        let reports = format!(
            "--setenv=KEYRINGD_TEST_REPORT_DIR={}",
            self.report_directory.display()
        );
        let authenticator = format!(
            "--setenv=KEYRINGD_E2E_AUTHENTICATOR_CONTROL={}",
            self.authenticator_control_url
        );
        let mut options = vec![
            "--setenv=USER=e2e".to_owned(),
            "--setenv=LOGNAME=e2e".to_owned(),
            "--setenv=KEYRINGD_E2E_ROOT=/var/lib/keyringd-e2e".to_owned(),
            "--setenv=KEYRINGD_E2E_SOFTWARE_TPM_RESET=1".to_owned(),
            artifacts,
            reports,
            authenticator,
            format!(
                "--setenv=KEYRINGD_TEST_USBIP_SERVER={}",
                super::virtual_authenticator::AUTHENTICATOR_USB_ADDRESS
            ),
            format!(
                "--setenv=KEYRINGD_E2E_USB_CONTROL={}",
                self.usb_control_socket.display()
            ),
        ];
        if let Some(bluetooth) = self.bluetooth {
            options.push(format!(
                "--setenv=KEYRINGD_E2E_BLE_SCANNER={}",
                bluetooth.scanner
            ));
            options.push(format!(
                "--setenv=KEYRINGD_E2E_BLE_ADVERTISER_ADDRESS={}",
                bluetooth.advertiser_address
            ));
        }
        if let Some(filter) = std::env::var_os("KEYRINGD_E2E_TRACE") {
            options.push(format!(
                "--setenv=KEYRINGD_E2E_TRACE={}",
                filter.to_string_lossy()
            ));
        }
        options.extend(
            APPLICATION_ENVIRONMENT
                .iter()
                .map(|assignment| format!("--setenv={assignment}")),
        );
        let executable = self.test_executable;
        let package = "keyringd-system-tests";
        let mut listing_arguments = vec![executable.as_os_str().to_os_string(), "--list".into()];
        if let Some(filter) = self.filter {
            listing_arguments.push(filter.into());
        }
        let listed = self.command.output_as_e2e_with_options(
            "E2E case listing",
            &options,
            listing_arguments,
        )?;
        let listed = std::str::from_utf8(&listed.stdout)?;
        ensure!(
            listed.lines().any(|line| line.ends_with(": test")),
            "no system test cases matched the filter"
        );
        let selection = keyringd_testkit::runner::ScenarioSelection::from_listing(listed)?;
        let mut arguments = vec![
            executable.as_os_str().to_os_string(),
            "--test-threads=1".into(),
        ];
        if let Some(filter) = self.filter {
            arguments.push(filter.into());
        }
        let executed = self
            .command
            .run_as_e2e_with_options("E2E tests", &options, arguments);
        let report_directory = self
            .workspace
            .join(self.report_directory.strip_prefix("/workspace")?);
        let verified = keyringd_testkit::runner::ScenarioResults::load(
            &report_directory,
            &format!("{package}::scenarios::"),
        )
        .and_then(|results| {
            println!("Linux system test scenarios: {results}");
            results.verify(&selection)
        });
        crate::util::finish([
            ("E2E test process", executed),
            ("E2E scenario results", verified),
        ])?;
        ensure!(!self.interrupted.load(Ordering::SeqCst), "interrupted");
        Ok(())
    }

    fn wait_until(&self, description: &str, mut ready: impl FnMut() -> Result<bool>) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            ensure!(!self.interrupted.load(Ordering::SeqCst), "interrupted");
            if ready()? {
                return Ok(());
            }
            ensure!(
                Instant::now() < deadline,
                "{description} did not become ready"
            );
            thread::sleep(Duration::from_millis(50));
        }
    }
}
