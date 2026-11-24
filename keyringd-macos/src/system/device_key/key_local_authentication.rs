use core_foundation::base::{CFEqual, CFGetTypeID, TCFType};
use keyringd_core::model::device_key::LocalAuthentication;
use security_framework::access_control::{ProtectionMode, SecAccessControl};
use security_framework::key::SecKey;
use security_framework::passwords::AccessControlOptions;
use security_framework_sys::item::kSecAttrAccessControl;

use super::secure_enclave_device_key_error::SecureEnclaveDeviceKeyError;

/// Match the actual key ACL against the two policies that this backend creates.
/// CFEqual compares SecAccessControl's policy dictionaries, not object identity.
pub(super) fn key_local_authentication(
    key: &SecKey,
) -> Result<LocalAuthentication, SecureEnclaveDeviceKeyError> {
    let attributes = key.attributes();
    let actual = *attributes
        .find(unsafe { kSecAttrAccessControl.cast() })
        .ok_or(SecureEnclaveDeviceKeyError::UnknownAccessControl)?;
    if unsafe { CFGetTypeID(actual) } != SecAccessControl::type_id() {
        return Err(SecureEnclaveDeviceKeyError::UnknownAccessControl);
    }
    for (flags, authentication) in [
        (
            AccessControlOptions::PRIVATE_KEY_USAGE | AccessControlOptions::USER_PRESENCE,
            LocalAuthentication::Required,
        ),
        (
            AccessControlOptions::PRIVATE_KEY_USAGE,
            LocalAuthentication::NotRequired,
        ),
    ] {
        let expected = SecAccessControl::create_with_protection(
            Some(ProtectionMode::AccessibleWhenUnlockedThisDeviceOnly),
            flags.bits(),
        )
        .map_err(|error| SecureEnclaveDeviceKeyError::SecurityFramework(error.into()))?;
        if unsafe { CFEqual(actual, expected.as_CFTypeRef()) } != 0 {
            return Ok(authentication);
        }
    }
    Err(SecureEnclaveDeviceKeyError::UnknownAccessControl)
}
