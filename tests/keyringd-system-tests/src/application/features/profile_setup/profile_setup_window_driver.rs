use super::ProfileSetupWizardDriver;
use anyhow::Result;

pub trait ProfileSetupWindowDriver {
    type Wizard<'a>: ProfileSetupWizardDriver
    where
        Self: 'a;
    fn screenshot(&self) -> Result<std::path::PathBuf>;
    fn capture_screenshot(&self, name: &str) -> Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    fn open(&mut self) -> Result<()>;
    fn close(&mut self) -> Result<()>;
    fn wizard(&mut self) -> Result<Self::Wizard<'_>>;
}
