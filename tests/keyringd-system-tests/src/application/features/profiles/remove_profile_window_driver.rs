use anyhow::Result;
use std::path::PathBuf;

pub trait RemoveProfileWindowDriver {
    fn screenshot(&self) -> Result<PathBuf>;
    fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    fn confirm(&mut self) -> Result<()>;
}
