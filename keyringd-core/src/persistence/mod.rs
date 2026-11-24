mod atomic_file_storage;
mod file_profile_repository;
mod file_save_error;
mod file_setting_repository;
mod profile_repository;
mod profile_save_error;
mod profile_validation_error;
mod profiles_document;
mod setting_document;
mod setting_repository;

pub use atomic_file_storage::AtomicFileStorage;
#[cfg(test)]
pub(crate) use atomic_file_storage::MockAtomicFileStorage;
pub use file_profile_repository::FileProfileRepository;
pub use file_save_error::FileSaveError;
pub use file_setting_repository::FileSettingRepository;
pub use profile_repository::ProfileRepository;
pub use profile_save_error::ProfileSaveError;
pub use profile_validation_error::ProfileValidationError;
pub use profiles_document::{PROFILES_SCHEMA_VERSION, ProfilesDocument};
pub use setting_document::SettingDocument;
#[cfg(test)]
pub(crate) use setting_repository::MockSettingRepository;
pub use setting_repository::SettingRepository;
