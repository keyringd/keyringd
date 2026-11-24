use anyhow::Result;
use std::path::PathBuf;

pub trait ReplaceDeviceKeyWindowDriver {
    type TrustEntriesWindow<'a>: super::TrustEntriesWindowDriver
    where
        Self: 'a;
    fn confirm(&self) -> Result<()>;
    fn press_use_security_key(&self) -> Result<()>;
    fn result_visible(&self) -> Result<bool>;
    fn result(&self) -> Result<super::DeviceKeyReplacementResult>;
    fn screenshot(&self) -> Result<PathBuf>;
    fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    fn open_trust_entries(&mut self, profile_name: &str) -> Result<Self::TrustEntriesWindow<'_>>;
}
