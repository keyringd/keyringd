use super::case_execution;

use crate::RunnerOptions;
use crate::build::{build_fixture, build_scenarios};
use crate::environment::{AuthenticatorEnvironment, Workspace};
use anyhow::Result;
use std::path::Path;

pub(crate) struct MacosExecution<'a> {
    workspace: &'a Workspace,
    options: &'a RunnerOptions,
}

impl<'a> MacosExecution<'a> {
    pub(crate) fn new(workspace: &'a Workspace, options: &'a RunnerOptions) -> Self {
        Self { workspace, options }
    }

    pub(crate) fn run(self) -> Result<()> {
        let started = std::time::Instant::now();
        let result = self.run_cases();
        if let Err(error) = &result {
            let report = &self.options.report_directory;
            let server_log = report.join("authenticator-server/authenticator.log");
            let artifacts = if server_log.is_file() {
                vec![server_log]
            } else {
                Vec::new()
            };
            keyringd_testkit::supervision::finish_case(
                report,
                "keyringd-system-tests-macos-runner::execution",
                &report.join("runner-checkpoint.json"),
                &self.workspace.root().join(
                    "tests/keyringd-system-tests-macos-runner/src/execution/macos_execution.rs",
                ),
                Some(format!("macOS UI runner failed: {error:#}")),
                started.elapsed().as_millis(),
                &artifacts,
            )
            .map_err(anyhow::Error::from_boxed)?;
        }
        result
    }

    fn run_cases(&self) -> Result<()> {
        let workspace = self.workspace;
        let options = self.options;
        let report_directory = &options.report_directory;
        use anyhow::{Context, ensure};

        for (name, executable, arguments) in [
            ("SSH client", "ssh", &["-V"][..]),
            ("SSH server", "/usr/sbin/sshd", &["-V"][..]),
        ] {
            keyringd_testkit::runner::ReportRunMetadata::record_tool(
                report_directory,
                keyringd_testkit::runner::ToolExecutionMetadata::capture(
                    name,
                    Path::new(executable),
                    arguments,
                    workspace.root(),
                ),
            )?;
        }
        let fixture = build_fixture(workspace)?;
        let cases = report_directory.join("cases");
        if cases.exists() {
            std::fs::remove_dir_all(&cases)?;
        }
        let authenticator = AuthenticatorEnvironment::prepare(workspace, report_directory)?;

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let client = authenticator.server.client();
        let scenarios = build_scenarios(workspace)?;
        let mut list = std::process::Command::new(&scenarios);
        list.arg("--list");
        if let Some(filter) = &options.filter {
            list.arg(filter);
        }
        let listed = list.output().context("failed to list macOS UI cases")?;
        ensure!(listed.status.success(), "macOS UI case listing failed");
        let listed = String::from_utf8_lossy(&listed.stdout);
        ensure!(
            listed.lines().any(|line| line.ends_with(": test")),
            "no system test cases matched the filter"
        );
        let selection = keyringd_testkit::runner::ScenarioSelection::from_listing(&listed)?;
        let expected = selection.cases();

        let mut failures = Vec::new();
        for (index, case) in expected.iter().enumerate() {
            println!("[{}/{}] {case}", index + 1, expected.len());
            let directory = report_directory
                .join("cases")
                .join(format!("{:04}", index + 1));
            let mut command = std::process::Command::new(&scenarios);
            command
                .current_dir(workspace.root())
                .args(["--exact", case, "--test-threads=1", "--nocapture"])
                .env("KEYRINGD_TEST_REPORT_DIR", report_directory)
                .env("KEYRINGD_UI_TEST_APP", &fixture)
                .env(
                    "KEYRINGD_TEST_AUTHENTICATOR_CONTROL",
                    format!("http://{}", authenticator.server.control_address()),
                )
                .env(
                    "KEYRINGD_TEST_USBIP_SERVER",
                    authenticator.server.usb_address().to_string(),
                )
                .env(
                    "KEYRINGD_UI_TEST_HYBRID_TUNNEL_ADDRESS",
                    authenticator
                        .server
                        .hybrid_address()
                        .context("owned server has no Hybrid tunnel")?
                        .to_string(),
                )
                .env(
                    "KEYRINGD_UI_TEST_HYBRID_TUNNEL_CA",
                    &authenticator.hybrid_ca,
                );
            let source = workspace
                .root()
                .join("tests/keyringd-system-tests/tests/main.rs");
            let passed = self::case_execution::CaseExecution::new(
                report_directory,
                &source,
                directory,
                &format!("keyringd-system-tests::scenarios::{case}"),
                std::time::Duration::from_secs(options.case_timeout_seconds),
            )
            .run(&mut command, |directory| {
                Self::clean_authenticators(&runtime, client, directory)
            })?;
            if !passed {
                failures.push(case.clone());
            }
        }
        let results = keyringd_testkit::runner::ScenarioResults::load(
            report_directory,
            "keyringd-system-tests::scenarios::",
        )?;
        println!("macOS UI scenarios: {results}");
        results.verify(&selection).and_then(|()| {
            ensure!(
                failures.is_empty(),
                "{} macOS UI cases failed: {}",
                failures.len(),
                failures.join(", ")
            );
            Ok(())
        })?;
        Ok(())
    }

    fn clean_authenticators(
        runtime: &tokio::runtime::Runtime,
        client: &keyringd_test_usbip::authenticator_server::client::AuthenticatorServerClient,
        directory: &Path,
    ) -> Result<Option<String>> {
        let remaining = runtime.block_on(client.authenticators())?;
        if remaining.is_empty() {
            return Ok(None);
        }
        let message = format!(
            "case left {} virtual authenticators behind",
            remaining.len()
        );
        // Keep attempting deletion even if writing evidence or deleting one key fails.
        let evidence = serde_json::to_vec_pretty(
            &remaining
                .iter()
                .map(|device| {
                    serde_json::json!({
                        "id": device.id, "bus_id": device.bus_id, "connected": device.connected,
                    })
                })
                .collect::<Vec<_>>(),
        )
        .map_err(anyhow::Error::from)
        .and_then(|bytes| {
            std::fs::write(directory.join("authenticator-residue.json"), bytes).map_err(Into::into)
        });
        let mut results = vec![("write residue evidence", evidence)];
        for device in remaining {
            let result =
                runtime.block_on(async { client.authenticator(device.id)?.delete().await });
            results.push((
                "delete virtual authenticator",
                result.map_err(anyhow::Error::from),
            ));
        }
        keyringd_testkit::runner::StageFailures::finish(results)
            .map_err(|error| error.context(message.clone()))?;
        Ok(Some(message))
    }
}
