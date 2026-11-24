use super::ControlledOpenSsh;
use anyhow::{Context, Result};
use keyringd_core::persistence::FileProfileRepository;
use keyringd_core::persistence::ProfileRepository;
use keyringd_core::test_support::{SaveFailureProfileRepository, TestFidoMode, TestPasskeyService};
use keyringd_ui_core::key_operation::KeyOperationCoordinator;

pub(super) struct UiTestControl<'a> {
    pub store: &'a SaveFailureProfileRepository<FileProfileRepository>,
    pub fido: &'a TestPasskeyService,
    pub ssh: &'a ControlledOpenSsh,
    pub pipe: &'a str,
    pub uses_test_passkey_service: bool,
    pub mock_interaction: &'a KeyOperationCoordinator,
}

impl UiTestControl<'_> {
    pub fn handle(&self, line: &str) -> Result<serde_json::Value> {
        let command: serde_json::Value = serde_json::from_str(line)?;
        anyhow::ensure!(
            self.uses_test_passkey_service
                || ["fido_mode", "fido_credentials", "export_fido_credentials"]
                    .iter()
                    .all(|key| command.get(key).is_none()),
            "FIDO response control requires the test PasskeyService"
        );
        if command
            .get("cancel_fido_operations")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        {
            self.mock_interaction.cancel_all_operations();
        }

        if let Some(mode) = command.get("fido_mode") {
            self.fido
                .set_mode(serde_json::from_value::<TestFidoMode>(mode.clone())?);
        }
        if let Some(fail) = command.get("fail_saves") {
            self.store
                .set_save_failure(fail.as_bool().context("fail_saves must be a boolean")?);
        }
        if let Some(config) = command.get("ssh_config") {
            self.ssh.set_configuration(
                config.as_str().context("ssh_config must be a string")?,
                self.pipe,
            );
        }
        if let Some(credentials) = command.get("fido_credentials") {
            self.fido
                .import_credentials(serde_json::from_value(credentials.clone())?)?;
        }
        if command
            .get("export_fido_credentials")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        {
            return Ok(serde_json::json!({"fido_credentials": self.fido.export_credentials()}));
        }
        let mut state = serde_json::json!({
            "agent_pipe": self.pipe,
            "test_passkey_service": self.uses_test_passkey_service,
            "profiles": self.store.load()?,
            "ssh_requests": self.ssh.requests(),
        });
        if self.uses_test_passkey_service {
            state["requests"] = self.fido.requests().into();
            state["pending"] = self.fido.pending().into();
            state["cancelled"] = self.fido.cancelled().into();
        }
        Ok(state)
    }
}
