use std::sync::Arc;

use super::{AtomicFileStorage, FileSaveError, SettingDocument, SettingRepository};

pub struct FileSettingRepository {
    file: Arc<dyn AtomicFileStorage>,
}

impl FileSettingRepository {
    pub fn new(file: Arc<dyn AtomicFileStorage>) -> Self {
        Self { file }
    }
}

impl SettingRepository for FileSettingRepository {
    fn load(&self) -> std::io::Result<SettingDocument> {
        match self.file.read()? {
            Some(value) => SettingDocument::from_json(&value),
            None => Ok(SettingDocument::default()),
        }
    }

    fn save(&self, document: &SettingDocument) -> Result<(), FileSaveError> {
        self.file.replace(&document.to_json()?)
    }
}
