use crate::application::platform::windows::UiDriver;
use anyhow::Result;
use windows::Win32::UI::Accessibility::IUIAutomationElement;

pub struct TrustEntriesWindow<'a> {
    pub(in crate::application::features) driver: &'a UiDriver,
    pub(in crate::application::features) window: IUIAutomationElement,
}

impl TrustEntriesWindow<'_> {
    pub fn entry_value(&self, title: &str) -> Result<String> {
        self.driver
            .text(&self.window, &format!("trust-entry-value-{title}"))
    }
    pub fn copy_entry(&self, title: &str) -> Result<String> {
        let sequence = self.driver.clipboard_sequence();
        self.driver
            .invoke_named(&self.window, &format!("Copy {title}"))?;
        self.driver.clipboard_after(sequence)
    }
    pub fn copy_direct_entries(&self) -> Result<String> {
        let sequence = self.driver.clipboard_sequence();
        self.driver
            .invoke_named(&self.window, "Copy Device + Passkey Public Keys")?;
        self.driver.clipboard_after(sequence)
    }

    pub fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    pub fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.driver.capture(&self.window)
    }

    pub fn close(&self) -> Result<()> {
        let window = unsafe { self.window.CurrentNativeWindowHandle()? };
        let owner = unsafe {
            windows::Win32::UI::WindowsAndMessaging::GetWindow(
                window,
                windows::Win32::UI::WindowsAndMessaging::GW_OWNER,
            )
        }?;
        let parent = self.driver.element_from_handle(owner.0 as usize)?;
        self.driver.invoke_named(&self.window, "Close")?;
        self.driver.wait_window_destroyed(window)?;
        self.driver.wait_window_enabled(&parent, true)?;
        // Modal teardown restores the owner's foreground after enabling it.
        // Finish that handoff before a different thread opens the tray popup.
        self.driver.activate_input_target(&parent)
    }
}

impl crate::application::features::profiles::TrustEntriesWindowDriver for TrustEntriesWindow<'_> {
    fn displayed_entries(&self) -> Result<Vec<String>> {
        [
            "Certificate Authority Public Key",
            "Device Key Public Key",
            "Passkey Public Key",
        ]
        .into_iter()
        .map(|name| self.entry_value(name))
        .collect()
    }
    fn copy_ca_trust_entry(&self) -> Result<String> {
        self.copy_entry("Certificate Authority Public Key")
    }
    fn copy_device_key_trust_entry(&self) -> Result<String> {
        self.copy_entry("Device Key Public Key")
    }
    fn copy_passkey_trust_entry(&self) -> Result<String> {
        self.copy_entry("Passkey Public Key")
    }
    fn copy_direct_key_trust_entries(&self) -> Result<String> {
        self.copy_direct_entries()
    }
    fn notifications(&self) -> Result<Vec<String>> {
        if !self
            .driver
            .contains_automation_id(&self.window, "feedback-toast-message")?
        {
            return Ok(Vec::new());
        }
        let element = self
            .driver
            .element(&self.window, "feedback-toast-message")?;
        if unsafe { element.CurrentIsOffscreen()?.as_bool() } {
            return Ok(Vec::new());
        }
        Ok(vec![unsafe { element.CurrentName()?.to_string() }])
    }
    fn screenshot(&self) -> Result<std::path::PathBuf> {
        TrustEntriesWindow::screenshot(self)
    }
    fn close(&mut self) -> Result<()> {
        TrustEntriesWindow::close(self)
    }
}
