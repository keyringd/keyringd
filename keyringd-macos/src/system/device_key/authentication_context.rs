use core_foundation::base::{CFType, TCFType};
use objc2::{
    msg_send,
    rc::Retained,
    runtime::{AnyClass, AnyObject},
};

use super::secure_enclave_device_key_error::SecureEnclaveDeviceKeyError;

#[link(name = "LocalAuthentication", kind = "framework")]
unsafe extern "C" {}

/// Each key lookup gets a fresh context; authentication is not reused across signatures.
pub(super) fn authentication_context() -> Result<CFType, SecureEnclaveDeviceKeyError> {
    let class = AnyClass::get(c"LAContext")
        .ok_or(SecureEnclaveDeviceKeyError::AuthenticationContextUnavailable)?;
    let context: Retained<AnyObject> = unsafe { msg_send![class, new] };
    let _: () =
        unsafe { msg_send![&*context, setTouchIDAuthenticationAllowableReuseDuration: 0.0_f64] };
    // Retain the Objective-C context for Security.framework's query dictionary.
    Ok(unsafe { CFType::wrap_under_get_rule(Retained::as_ptr(&context).cast()) })
}
