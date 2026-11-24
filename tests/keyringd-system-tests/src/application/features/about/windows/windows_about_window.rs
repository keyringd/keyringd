use crate::application::platform::windows::UiDriver;
use anyhow::Result;
use windows::Win32::UI::Accessibility::IUIAutomationElement;

pub struct WindowsAboutWindow<'a> {
    pub(in crate::application::features) driver: &'a UiDriver,
    pub(in crate::application::features) pid: u32,
}

impl<'a> WindowsAboutWindow<'a> {
    pub fn open_from_background_menu(&mut self) -> Result<()> {
        crate::application::features::navigation::windows::UiApplication::new(
            self.driver,
            self.pid,
        )
        .open_main_menu()?
        .open_about()?;
        self.window()?;
        Ok(())
    }

    pub fn new(driver: &'a UiDriver, pid: u32) -> Self {
        Self { driver, pid }
    }
    fn window(&self) -> Result<IUIAutomationElement> {
        self.driver.window_named(self.pid, "About Keyringd")
    }

    fn read_application_info(
        &self,
    ) -> Result<crate::application::features::about::AboutApplicationInfo> {
        use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
        use windows::Win32::System::Variant::VARIANT;
        use windows::Win32::UI::Accessibility::{
            CUIAutomation, IUIAutomation, TreeScope_Descendants, UIA_ControlTypePropertyId,
            UIA_TextControlTypeId,
        };
        let mut texts = Vec::new();
        let mut enabled_actions = Vec::new();
        unsafe {
            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
            let condition = automation.CreatePropertyCondition(
                UIA_ControlTypePropertyId,
                &VARIANT::from(UIA_TextControlTypeId.0),
            )?;
            let elements = self.window()?.FindAll(TreeScope_Descendants, &condition)?;
            for index in 0..elements.Length()? {
                let element = elements.GetElement(index)?;
                if !element.CurrentIsOffscreen()?.as_bool() {
                    texts.push(element.CurrentName()?.to_string());
                }
            }
            for name in ["Website", "Report an issue"] {
                let element = self.driver.element_named(&self.window()?, name)?;
                if !element.CurrentIsOffscreen()?.as_bool() && element.CurrentIsEnabled()?.as_bool()
                {
                    enabled_actions.push(element.CurrentName()?.to_string());
                }
            }
        }
        Ok(crate::application::features::about::AboutApplicationInfo {
            texts,
            enabled_actions,
        })
    }
    fn open_document(
        &self,
        button: &str,
        title: &str,
    ) -> Result<crate::application::features::about::windows::WindowsLicenseWindow<'a>> {
        self.driver.press_named_enter(&self.window()?, button)?;
        let window = self.driver.window_named(self.pid, title)?;
        self.driver.wait_window_enabled(&self.window()?, false)?;
        Ok(
            crate::application::features::about::windows::WindowsLicenseWindow {
                driver: self.driver,
                window,
                owner: self.window()?,
            },
        )
    }
    fn open_application_document(
        &self,
    ) -> Result<crate::application::features::about::windows::WindowsLicenseWindow<'a>> {
        self.open_document("License…", "Keyringd License")
    }
    fn open_third_party_document(
        &self,
    ) -> Result<crate::application::features::about::windows::WindowsLicenseWindow<'a>> {
        self.open_document("Third-party Licenses…", "Third-party Licenses")
    }
    fn capture(&self) -> Result<std::path::PathBuf> {
        self.driver.capture(&self.window()?)
    }
    fn close_window(&self) -> Result<()> {
        self.driver.invoke_named(&self.window()?, "Close")?;
        self.driver.wait_window_absent(self.pid, "About Keyringd")
    }
}

impl super::super::AboutWindowDriver for WindowsAboutWindow<'_> {
    type ApplicationLicense<'a>
        = super::WindowsLicenseWindow<'a>
    where
        Self: 'a;
    type ThirdPartyLicensesWindow<'a>
        = super::WindowsLicenseWindow<'a>
    where
        Self: 'a;
    fn open(&mut self) -> Result<()> {
        self.open_from_background_menu()
    }
    fn screenshot(&self) -> Result<std::path::PathBuf> {
        self.capture()
    }
    fn application_info(&self) -> Result<super::super::AboutApplicationInfo> {
        self.read_application_info()
    }
    fn open_application_license(&mut self) -> Result<Self::ApplicationLicense<'_>> {
        self.open_application_document()
    }
    fn open_third_party_licenses(&mut self) -> Result<Self::ThirdPartyLicensesWindow<'_>> {
        self.open_third_party_document()
    }
    fn close(&mut self) -> Result<()> {
        self.close_window()
    }
}
