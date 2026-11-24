use anyhow::Result;
use std::path::PathBuf;
pub trait ProfileSetupPinViewDriver {
    fn visible(&self, new_pin: bool) -> Result<bool>;
    fn content(&self) -> Result<super::ProfileSetupPinContent>;
    fn screenshot(&self) -> Result<PathBuf>;
    fn capture_screenshot(&self, name: &str) -> Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    fn enter(&mut self, pin: &str, new_pin: bool) -> Result<()>;
}
