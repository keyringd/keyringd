use super::super::{SshConnectionIdentity, SshConnectionTestWindowDriver};
use crate::application::features::profiles::macos::MacosProfilesWindow;
use crate::application::platform::macos::UiDriver;
use crate::environment::macos::TestApp;
use anyhow::Result;
use std::path::PathBuf;
const HOST: &str = "connection-test.host";
const TEST: &str = "connection-test.test";
const CANCEL: &str = "connection-test.cancel";

pub struct MacosSshConnectionTestWindow<'a> {
    pub(crate) app: &'a TestApp,
}
impl MacosSshConnectionTestWindow<'_> {
    fn driver(&self) -> Result<UiDriver<'_>> {
        let driver = UiDriver::new(self.app).in_window(HOST);
        driver.wait_for(HOST)?;
        Ok(driver)
    }
    fn wait_selected_identity(&self, identity: SshConnectionIdentity) -> Result<()> {
        let driver = self.driver()?;
        for choice in [
            SshConnectionIdentity::DeviceCertificate,
            SshConnectionIdentity::DeviceKey,
            SshConnectionIdentity::Passkey,
        ] {
            driver.wait_value(
                choice.identifier(),
                if choice.identifier() == identity.identifier() {
                    "1"
                } else {
                    "0"
                },
            )?;
        }
        Ok(())
    }
}
impl SshConnectionTestWindowDriver for MacosSshConnectionTestWindow<'_> {
    fn open(&mut self, profile: &str) -> Result<()> {
        anyhow::ensure!(
            UiDriver::new(self.app)
                .find_element(HOST)?
                .is_none_or(|element| element.hidden == Some(true)),
            "a connection test is already open"
        );
        MacosProfilesWindow::new(&UiDriver::new(self.app))?.open_connection_test(profile)?;
        self.driver()?;
        Ok(())
    }
    fn selected_identities(&self) -> Result<Vec<SshConnectionIdentity>> {
        let driver = self.driver()?;
        let mut selected = Vec::new();
        for choice in [
            SshConnectionIdentity::DeviceCertificate,
            SshConnectionIdentity::DeviceKey,
            SshConnectionIdentity::Passkey,
        ] {
            let element = driver.element(choice.identifier())?;
            match element.value.as_deref() {
                Some("1") => selected.push(choice),
                Some("0") => {}
                other => anyhow::bail!("identity has no boolean selection state: {other:?}"),
            }
        }
        Ok(selected)
    }
    fn select_identity(&self, identity: SshConnectionIdentity) -> Result<()> {
        let driver = self.driver()?;
        let identifier = identity.identifier();
        driver.press(identifier)?;
        driver.wait_value(identifier, "1")?;
        self.wait_selected_identity(identity)
    }
    fn screenshot(&self) -> Result<PathBuf> {
        let driver = self.driver()?;
        driver.capture(HOST)
    }
    fn start(&self, host: &str) -> Result<()> {
        let driver = self.driver()?;
        let entered = driver.enter_text(HOST, host)?;
        anyhow::ensure!(
            entered == host,
            "the host field holds {entered:?} after entering {host:?}"
        );
        // Wait for the state machine to process the host change, which enables
        // the Test button.
        driver.wait_until(TEST, "to become enabled", |element| {
            element.enabled == Some(true)
        })?;
        driver.press(TEST)
    }
    fn close(&mut self) -> Result<()> {
        let driver = self.driver()?;
        driver.press(CANCEL)?;
        driver.wait_absent(HOST)?;
        Ok(())
    }
}
