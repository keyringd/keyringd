use super::ProfileSaveError;
use super::ProfilesDocument;
use crate::model::Profile;

/// Loads and saves persisted profiles; live application state belongs to ProfileStore.
pub trait ProfileRepository: Send + Sync {
    fn load(&self) -> std::io::Result<Vec<Profile>>;
    /// Validates the document before replacing the persisted profiles.
    fn save(&self, document: &ProfilesDocument) -> Result<(), ProfileSaveError>;
}
