use crate::application::features::ssh_connection_test::{
    ConnectionTestResultWindowDriver, SshConnectionResult,
};
use crate::application::platform::linux::{ELEMENT_TIMEOUT, KEY_OPERATION_TIMEOUT, UiDriver};
use anyhow::Result;
use std::path::PathBuf;
use tokio::runtime::Runtime;

pub struct LinuxConnectionTestResultWindow<'a> {
    pub(crate) runtime: &'a Runtime,
    pub(crate) session: UiDriver,
}
impl LinuxConnectionTestResultWindow<'_> {
    async fn result_session(&self) -> Result<UiDriver> {
        let (session, _) = self
            .session
            .application()
            .wait_for_dialog_body("Host: ", KEY_OPERATION_TIMEOUT)
            .await?;
        Ok(session)
    }
}
impl ConnectionTestResultWindowDriver for LinuxConnectionTestResultWindow<'_> {
    fn visible(&self) -> Result<bool> {
        self.runtime.block_on(async {
            // The result MessageDialog body has the same Host prefix used by
            // result_session, for both successful and unsuccessful connections.
            self.session
                .application()
                .visible_name_starting_with("Host: ")
                .await
        })
    }
    fn content(&self) -> Result<SshConnectionResult> {
        self.runtime.block_on(async {
            use crate::application::features::ssh_connection_test::SshConnectionResult;
            use crate::application::features::ssh_connection_test::SshRejectionGuidance;
            let session = self.result_session().await?;
            let (_, body) = session
                .wait_for_dialog_body("Host: ", ELEMENT_TIMEOUT)
                .await?;
            let labels = session
                .preceding_sibling_names_with_role(&body, atspi::Role::Label)
                .await?;
            let (host, diagnostic) = body
                .strip_prefix("Host: ")
                .unwrap()
                .split_once("\n\n")
                .ok_or_else(|| {
                    anyhow::anyhow!("SSH result body does not separate host and diagnostic: {body}")
                })?;
            let groups = session
                .visible_names_with_role(atspi::Role::Grouping)
                .await?;
            let rejection = if groups.iter().any(|name| name == "SSH rejection guidance") {
                let guidance = session
                    .wait_for_group("SSH rejection guidance", ELEMENT_TIMEOUT)
                    .await?;
                let entries = guidance.visible_texts_with_role(atspi::Role::Text).await?;
                anyhow::ensure!(
                    entries.len() == 1,
                    "unexpected SSH registration text fields: {entries:?}"
                );
                Some(SshRejectionGuidance {
                    entry: entries[0].clone(),
                    instructions: guidance
                        .visible_names_with_role(atspi::Role::Label)
                        .await?
                        .join("\n"),
                })
            } else {
                None
            };
            let titles = labels
                .iter()
                .filter(|name| !name.is_empty() && **name != body)
                .collect::<Vec<_>>();
            anyhow::ensure!(
                titles.len() == 1,
                "SSH result heading was ambiguous: {titles:?}"
            );
            Ok(SshConnectionResult {
                title: titles[0].clone(),
                host: host.to_owned(),
                diagnostic: diagnostic.to_owned(),
                rejection,
            })
        })
    }
    fn copy_rejected_entry(&self) -> Result<String> {
        self.runtime.block_on(async {
            let session = self.result_session().await?;
            let copy = session
                .wait_for_enabled_action("Copy Entry", ELEMENT_TIMEOUT)
                .await?;
            session.copy_to_clipboard(&copy).await?;
            session.clipboard_text().await
        })
    }
    fn screenshot(&self) -> Result<PathBuf> {
        self.runtime
            .block_on(async { self.result_session().await?.capture().await })
    }

    fn close(&self) -> Result<()> {
        self.runtime.block_on(async {
            let session = self.result_session().await?;
            let close = session
                .wait_for_enabled_action("Close", ELEMENT_TIMEOUT)
                .await?;
            session.click(&close).await?;
            self.session
                .application()
                .wait_for_enabled_action("Test", ELEMENT_TIMEOUT)
                .await?;
            Ok(())
        })
    }
}
