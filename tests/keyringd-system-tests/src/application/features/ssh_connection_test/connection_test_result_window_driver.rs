use super::SshConnectionResult;
use anyhow::Result;
use std::path::PathBuf;
pub trait ConnectionTestResultWindowDriver {
    fn visible(&self) -> Result<bool>;
    fn content(&self) -> Result<SshConnectionResult>;
    fn copy_rejected_entry(&self) -> Result<String>;
    fn screenshot(&self) -> Result<PathBuf>;
    fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    fn close(&self) -> Result<()>;
}
