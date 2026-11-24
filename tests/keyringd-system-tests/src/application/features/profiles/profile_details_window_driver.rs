use anyhow::Result;
use std::path::PathBuf;

pub trait ProfileDetailsWindowDriver {
    type TrustEntriesWindow<'a>: super::TrustEntriesWindowDriver
    where
        Self: 'a;
    type ReplaceDeviceKeyWindow<'a>: super::ReplaceDeviceKeyWindowDriver
    where
        Self: 'a;
    fn content(&self) -> Result<super::ProfileDetails>;
    fn screenshot(&self) -> Result<PathBuf>;
    fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    fn close(&mut self) -> Result<()>;
    fn open_trust_entries(&mut self) -> Result<Self::TrustEntriesWindow<'_>>;
    fn open_replacement(&mut self) -> Result<Self::ReplaceDeviceKeyWindow<'_>>;
}
