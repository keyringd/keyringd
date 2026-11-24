use anyhow::Result;
use std::path::PathBuf;
pub trait SecurityKeyWindowDriver {
    fn wait_open(&self) -> Result<()>;
    fn screenshot(&self) -> Result<PathBuf>;
    fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    fn wait_closed(&self) -> Result<()>;
    #[cfg(windows)]
    fn pin_visible(&self) -> Result<bool>;
    #[cfg(windows)]
    fn enter_pin(&self, pin: &str) -> Result<()>;
}
