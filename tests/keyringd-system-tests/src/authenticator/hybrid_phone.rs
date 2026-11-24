use super::AuthenticatorControl;
use anyhow::Result;
use keyringd_test_usbip::authenticator_server::client::{AuthenticatorHandle, HybridAdvertisement};
use std::rc::Rc;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// A case-owned phone, with no USB import. Clones share its credentials.
#[derive(Clone)]
pub struct HybridPhone {
    authenticator: Rc<AuthenticatorControl>,
}

impl HybridPhone {
    pub(crate) fn new(runtime: Arc<Runtime>, handle: AuthenticatorHandle) -> Self {
        Self {
            authenticator: Rc::new(AuthenticatorControl::new(runtime, handle)),
        }
    }

    pub fn authenticator(&self) -> &AuthenticatorControl {
        &self.authenticator
    }

    pub fn scan_hybrid_qr(&self, uri: &str) -> Result<HybridAdvertisement> {
        Ok(self
            .authenticator
            .runtime
            .block_on(self.authenticator.handle.scan_hybrid_qr(uri))?)
    }
}

impl std::fmt::Debug for HybridPhone {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HybridPhone")
            .field("id", &self.authenticator.id())
            .finish()
    }
}
