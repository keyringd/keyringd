use anyhow::Result;
use std::path::PathBuf;

pub trait TrustEntriesWindowDriver {
    fn displayed_entries(&self) -> Result<Vec<String>>;
    fn copy_ca_trust_entry(&self) -> Result<String>;
    fn copy_device_key_trust_entry(&self) -> Result<String>;
    fn copy_passkey_trust_entry(&self) -> Result<String>;
    fn copy_direct_key_trust_entries(&self) -> Result<String>;
    fn notifications(&self) -> Result<Vec<String>>;
    fn screenshot(&self) -> Result<PathBuf>;
    fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    fn close(&mut self) -> Result<()>;
}
