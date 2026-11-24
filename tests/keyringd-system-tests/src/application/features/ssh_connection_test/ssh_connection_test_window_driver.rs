use super::SshConnectionIdentity;
use anyhow::Result;
use std::path::PathBuf;
pub trait SshConnectionTestWindowDriver {
    fn open(&mut self, profile: &str) -> Result<()>;
    fn selected_identities(&self) -> Result<Vec<SshConnectionIdentity>>;
    fn select_identity(&self, identity: SshConnectionIdentity) -> Result<()>;
    fn start(&self, host: &str) -> Result<()>;
    fn screenshot(&self) -> Result<PathBuf>;
    fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    fn close(&mut self) -> Result<()>;
}
