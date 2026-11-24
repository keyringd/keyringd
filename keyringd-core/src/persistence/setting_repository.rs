use super::{FileSaveError, SettingDocument};

#[cfg_attr(test, mockall::automock)]
pub trait SettingRepository: Send + Sync {
    fn load(&self) -> std::io::Result<SettingDocument>;
    fn save(&self, document: &SettingDocument) -> Result<(), FileSaveError>;
}
