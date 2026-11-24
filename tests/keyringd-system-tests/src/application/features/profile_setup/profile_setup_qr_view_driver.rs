use anyhow::Result;
use std::path::PathBuf;

pub trait ProfileSetupQrViewDriver {
    fn content(&self) -> Result<super::ProfileSetupProgress>;
    fn press_use_security_key(&self) -> Result<()>;
    fn read_hybrid_qr(&self, previous: Option<&str>) -> Result<(String, PathBuf)>;
}
