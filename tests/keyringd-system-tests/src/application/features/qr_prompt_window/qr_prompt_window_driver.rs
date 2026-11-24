use anyhow::Result;
use std::path::PathBuf;
pub trait QrPromptWindowDriver {
    fn read_hybrid_qr(&self) -> Result<(String, PathBuf)>;
    fn security_key_enabled(&self) -> Result<bool>;
    fn press_use_security_key(&self) -> Result<()>;
    fn screenshot(&self) -> Result<PathBuf>;
    fn capture_screenshot(&self, name: &str) -> anyhow::Result<()> {
        keyringd_testkit::record_image(name, self.screenshot()?)
    }
    fn wait_closed(&self) -> Result<()>;
}
