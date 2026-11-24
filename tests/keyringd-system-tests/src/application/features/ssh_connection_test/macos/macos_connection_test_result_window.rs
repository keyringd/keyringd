use super::super::ConnectionTestResultWindowDriver;
use crate::application::platform::macos::UiDriver;
use crate::environment::macos::TestApp;
use anyhow::Result;
use std::path::PathBuf;
const RESULT: &str = "connection-test.result";
const CLOSE: &str = "connection-test.result.close";

pub struct MacosConnectionTestResultWindow<'a> {
    pub(crate) app: &'a TestApp,
}
impl MacosConnectionTestResultWindow<'_> {
    fn driver(&self) -> UiDriver<'_> {
        UiDriver::new(self.app)
            .in_window("connection-test.host")
            .in_sheet("connection-test.host")
    }
}
impl ConnectionTestResultWindowDriver for MacosConnectionTestResultWindow<'_> {
    fn content(
        &self,
    ) -> Result<crate::application::features::ssh_connection_test::SshConnectionResult> {
        let driver = self.driver();
        driver.wait_for(RESULT)?;
        use crate::application::features::ssh_connection_test::SshConnectionResult;
        use crate::application::features::ssh_connection_test::SshRejectionGuidance;
        driver.wait_until(CLOSE, "to allow closing the result", |element| {
            element.enabled == Some(true)
        })?;
        let text = |id: &str| -> Result<String> {
            driver
                .wait_for(&format!("connection-test.result.{id}"))?
                .value
                .ok_or_else(|| anyhow::anyhow!("result {id} has no accessible text"))
        };
        let body = text("body")?;
        let (host, diagnostic) = body
            .strip_prefix("Host: ")
            .and_then(|body| body.split_once("\n\n"))
            .ok_or_else(|| anyhow::anyhow!("invalid connection result body: {body:?}"))?;
        let rejection = if driver
            .find_element("connection-test.result.copy")?
            .is_some()
        {
            Some(SshRejectionGuidance {
                entry: text("entry")?,
                instructions: [text("authorized-keys")?, text("server")?].join("\n"),
            })
        } else {
            None
        };
        Ok(SshConnectionResult {
            title: text("title")?,
            host: host.to_owned(),
            diagnostic: diagnostic.to_owned(),
            rejection,
        })
    }
    fn visible(&self) -> Result<bool> {
        let driver = self.driver();
        Ok(driver
            .find_element(RESULT)?
            .is_some_and(|e| e.hidden != Some(true)))
    }
    fn copy_rejected_entry(&self) -> Result<String> {
        let driver = self.driver();
        driver.clear_pasteboard()?;
        driver.press("connection-test.result.copy")?;
        driver.wait_pasteboard_text()
    }
    fn close(&self) -> Result<()> {
        let driver = self.driver();
        driver.press(CLOSE)?;
        driver.wait_absent(RESULT)?;
        driver.in_window("connection-test.host").wait_until(
            "connection-test.test",
            "to allow another test",
            |element| element.enabled == Some(true),
        )?;
        Ok(())
    }
    fn screenshot(&self) -> Result<PathBuf> {
        let driver = self.driver();
        driver.capture_current_window()
    }
}
