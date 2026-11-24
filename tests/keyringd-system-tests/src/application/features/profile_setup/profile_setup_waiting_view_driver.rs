use anyhow::Result;
use std::path::PathBuf;

pub trait ProfileSetupWaitingViewDriver {
    fn content(&self) -> Result<Vec<String>>;
    fn screenshot(&self) -> Result<PathBuf>;
    fn capture_screenshot(&self, name: &str) -> Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
}
