use super::LicenseDocumentDriver;
use anyhow::Result;
pub trait LicenseWindowDriver: LicenseDocumentDriver {
    fn close(&self) -> Result<()>;
}
