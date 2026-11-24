use crate::text;
use keyringd_core::model::device_key::LocalAuthentication;
use keyringd_core::service::DeviceKeyProtectionChange;

pub fn operation(change: DeviceKeyProtectionChange) -> &'static str {
    match change {
        DeviceKeyProtectionChange::Enable => text::DEVICE_KEY_PROTECTION_ENABLE_OPERATION,
        DeviceKeyProtectionChange::ChangePassword => text::DEVICE_KEY_PROTECTION_CHANGE_OPERATION,
        DeviceKeyProtectionChange::Disable => text::DEVICE_KEY_PROTECTION_DISABLE_OPERATION,
    }
}

pub fn heading(name: &str, change: DeviceKeyProtectionChange) -> String {
    format!("{} for “{name}”?", operation(change))
}

pub fn confirmation(explanation: &str) -> String {
    format!(
        "{explanation}\n\nThe current key remains active unless the change finishes successfully."
    )
}

pub fn succeeded(change: DeviceKeyProtectionChange) -> &'static str {
    match change {
        DeviceKeyProtectionChange::Enable => text::DEVICE_KEY_PROTECTION_ENABLED_NOTICE,
        DeviceKeyProtectionChange::ChangePassword => text::DEVICE_KEY_PROTECTION_UPDATED_NOTICE,
        DeviceKeyProtectionChange::Disable => text::DEVICE_KEY_PROTECTION_DISABLED_NOTICE,
    }
}

pub fn authentication_label(authentication: LocalAuthentication) -> &'static str {
    match authentication {
        LocalAuthentication::Required => text::LOCAL_AUTHENTICATION_REQUIRED,
        LocalAuthentication::NotRequired => text::LOCAL_AUTHENTICATION_NOT_REQUIRED,
    }
}
