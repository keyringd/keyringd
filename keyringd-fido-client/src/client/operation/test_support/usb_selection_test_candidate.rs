use super::super::ctap::CtapOperationExecutor;
use crate::*;

use keyringd_fido::AuthenticatorTransport;

use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use super::*;
use keyringd_fido::ctap::{AuthenticatorGetInfoResponse, CtapVersion};
use keyringd_fido::{CtapCommand, CtapStatusCode};

/// The CTAP execution part waiting to be selected and the transmission/cancellation record that can be confirmed directly on the test side.
pub(crate) struct UsbSelectionTestCandidate {
    pub(crate) executor: CtapOperationExecutor,
    pub(crate) commands: Arc<Mutex<Vec<CtapCommand>>>,
    pub(crate) touch: Arc<tokio::sync::Notify>,
    pub(crate) cancelled: Arc<AtomicBool>,
}
impl UsbSelectionTestCandidate {
    pub(crate) fn new(version: CtapVersion, status: CtapStatusCode) -> Self {
        let commands = Arc::new(Mutex::new(Vec::new()));
        let touch = Arc::new(tokio::sync::Notify::new());
        let cancelled = Arc::new(AtomicBool::new(false));
        let channel = selection_transport(
            built_in_uv_transport(
                Arc::new(
                    keyringd_test_authenticator::ModelCtapAuthenticator::with_pin_uv_auth_token(
                        vec![0x5a; 32],
                    ),
                ),
                crate::ctap::test_support::TestPinUvKeyAgreement::new(PinUvAuthProtocol::Two),
                Arc::new(Mutex::new(Vec::new())),
                0,
                CtapStatusCode::UserVerificationInvalid,
                8,
            ),
            Arc::clone(&commands),
            Arc::clone(&touch),
            Arc::clone(&cancelled),
            version,
            status,
        );
        let capabilities = AuthenticatorCapabilities::from_get_info(
            AuthenticatorGetInfoResponse::new(vec![version.as_str().to_owned()], [0; 16])
                .with_options(BTreeMap::from([
                    ("rk".to_owned(), true),
                    ("clientPin".to_owned(), true),
                    ("pinUvAuthToken".to_owned(), true),
                ]))
                .with_pin_uv_auth_protocols(vec![2]),
        );
        Self {
            executor: CtapOperationExecutor::new(
                channel,
                AuthenticatorTransport::usb(),
                capabilities,
            ),
            commands,
            touch,
            cancelled,
        }
    }
}
