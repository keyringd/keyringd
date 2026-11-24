use std::path::PathBuf;

use anyhow::Result;

use crate::application::platform::macos::UiDriver;

const HEADING: &str = "trust-entries.heading";
const COPY_CA: &str = "trust-entries.copy-ca";
const COPY_DEVICE_KEY: &str = "trust-entries.copy-device-key";
const COPY_PASSKEY: &str = "trust-entries.copy-passkey";
const COPY_DIRECT_KEYS: &str = "trust-entries.copy-direct-keys";
const CLOSE: &str = "trust-entries.close";

/// The product Public Keys window.
///
/// Product copy actions write to a private pasteboard installed by the fixture.
/// The user's clipboard is neither cleared nor read.
pub struct TrustEntriesWindow<'a> {
    driver: UiDriver<'a>,
}

impl<'a> TrustEntriesWindow<'a> {
    pub fn wait_for(driver: &UiDriver<'a>, profile_name: &str) -> Result<Self> {
        let page = Self {
            driver: driver.in_window(HEADING),
        };
        page.driver
            .wait_value(HEADING, &format!("Public Keys for “{profile_name}”"))?;
        Ok(page)
    }

    pub fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn screenshot(&self) -> Result<PathBuf> {
        self.driver.capture(HEADING)
    }

    pub fn copy_ca_trust_entry(&self) -> Result<String> {
        self.copy(COPY_CA)
    }

    pub fn copy_device_key_trust_entry(&self) -> Result<String> {
        self.copy(COPY_DEVICE_KEY)
    }

    pub fn copy_passkey_trust_entry(&self) -> Result<String> {
        self.copy(COPY_PASSKEY)
    }

    pub fn copy_direct_key_trust_entries(&self) -> Result<String> {
        self.copy(COPY_DIRECT_KEYS)
    }

    pub fn displayed_entries(&self) -> Result<Vec<String>> {
        [COPY_CA, COPY_DEVICE_KEY, COPY_PASSKEY]
            .into_iter()
            .map(|id| {
                self.driver
                    .element(&format!("{id}.value"))?
                    .value
                    .ok_or_else(|| anyhow::anyhow!("trust entry {id} has no displayed value"))
            })
            .collect()
    }
    pub fn close(&self) -> Result<()> {
        self.driver.press(CLOSE)?;
        self.driver.wait_absent(HEADING)
    }

    /// Presses a copy button and reads back what reached the pasteboard, which
    /// is the only place the copied text can be observed. Clearing first means
    /// the read cannot mistake an earlier copy — or the user's own clipboard —
    /// for this one.
    fn copy(&self, identifier: &str) -> Result<String> {
        self.driver.clear_pasteboard()?;
        self.driver.press(identifier)?;
        self.driver.wait_pasteboard_text()
    }
}

impl crate::application::features::profiles::TrustEntriesWindowDriver for TrustEntriesWindow<'_> {
    fn displayed_entries(&self) -> Result<Vec<String>> {
        TrustEntriesWindow::displayed_entries(self)
    }
    fn copy_ca_trust_entry(&self) -> Result<String> {
        TrustEntriesWindow::copy_ca_trust_entry(self)
    }
    fn copy_device_key_trust_entry(&self) -> Result<String> {
        TrustEntriesWindow::copy_device_key_trust_entry(self)
    }
    fn copy_passkey_trust_entry(&self) -> Result<String> {
        TrustEntriesWindow::copy_passkey_trust_entry(self)
    }
    fn copy_direct_key_trust_entries(&self) -> Result<String> {
        TrustEntriesWindow::copy_direct_key_trust_entries(self)
    }
    fn notifications(&self) -> Result<Vec<String>> {
        Ok(self
            .driver
            .find_element("feedback.toast-message")?
            .filter(|element| element.hidden != Some(true))
            .and_then(|element| element.value)
            .into_iter()
            .collect())
    }
    fn screenshot(&self) -> Result<PathBuf> {
        TrustEntriesWindow::screenshot(self)
    }
    fn close(&mut self) -> Result<()> {
        TrustEntriesWindow::close(self)
    }
}
