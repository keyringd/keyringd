use super::DeviceKeyMaterial;
use crate::model::device_key::DevicePrivateKeyHandle;

/// Backend-defined cleanup for a successful protection change.
/// Cleanup removes unused key resources; it does not roll back a policy change.
pub struct DeviceKeyProtectionChangeResult {
    pub updated_key: DeviceKeyMaterial,
    /// Delete only after the updated Profile has been saved and published.
    pub delete_after_commit: Option<DevicePrivateKeyHandle>,
    /// Delete if certificate preparation or Profile persistence fails.
    pub delete_on_failure: Option<DevicePrivateKeyHandle>,
}

impl DeviceKeyProtectionChangeResult {
    /// An updated key with no separately owned resources to clean up.
    pub fn updated(updated_key: DeviceKeyMaterial) -> Self {
        Self {
            updated_key,
            delete_after_commit: None,
            delete_on_failure: None,
        }
    }

    /// A replacement resource: keep the original until the new Profile is saved.
    pub fn replacement(
        updated_key: DeviceKeyMaterial,
        previous_handle: DevicePrivateKeyHandle,
    ) -> Self {
        Self {
            delete_after_commit: Some(previous_handle),
            delete_on_failure: Some(updated_key.handle.clone()),
            updated_key,
        }
    }
}
