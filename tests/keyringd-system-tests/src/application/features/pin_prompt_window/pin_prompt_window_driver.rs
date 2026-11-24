use anyhow::Result;
use std::path::PathBuf;
pub trait PinPromptWindowDriver {
    fn screenshot(&self) -> Result<PathBuf>;
    fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    fn enter_pin(&self, pin: &str) -> Result<()>;
    fn submit(&self) -> Result<()>;
    fn wait_closed(&self) -> Result<()>;
}
