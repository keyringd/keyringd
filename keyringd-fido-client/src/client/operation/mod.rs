// Unit tests inject the platform route; native candidate paths are compiled separately.
#![cfg_attr(test, allow(dead_code))]
mod active_operation_guard;
mod authenticator_broker;
pub(crate) mod ctap;
mod get_assertion_operation_context;
mod make_credential_operation_context;
mod operation_context;
mod operation_id;
mod operation_manager;

pub(crate) use authenticator_broker::AuthenticatorBroker;
pub use ctap::ResponseValidationError;
pub(crate) use get_assertion_operation_context::GetAssertionOperationContext;
pub(crate) use make_credential_operation_context::MakeCredentialOperationContext;
pub use operation_id::OperationId;
pub(crate) use operation_manager::OperationManager;

#[cfg(test)]
pub(crate) use operation_context::OperationContext;

pub(crate) use ctap::SelectedUserVerification;

use active_operation_guard::ActiveOperationGuard;

mod hybrid_operation_executor;
#[cfg(test)]
pub(crate) mod test_support;
mod usb_authenticator_selector;
mod usb_operation_executor;
use hybrid_operation_executor::HybridOperationExecutor;
use usb_authenticator_selector::UsbAuthenticatorSelector;
use usb_operation_executor::UsbOperationExecutor;
