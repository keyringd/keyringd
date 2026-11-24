use super::ApprovalMode;
use anyhow::Result;
use keyringd_test_usbip::authenticator_server::client::{
    AuthenticatorHandle, AuthenticatorInfo, AuthenticatorProfile, ConsentKind, PendingConsent,
};
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Synchronous control of a case-owned authenticator, shared by USB keys and Hybrid phones.
/// The enclosing device's last owner deletes the authenticator from the server.
pub struct AuthenticatorControl {
    pub(super) runtime: Arc<Runtime>,
    pub(super) handle: AuthenticatorHandle,
}

impl AuthenticatorControl {
    pub(super) fn new(runtime: Arc<Runtime>, handle: AuthenticatorHandle) -> Self {
        Self { runtime, handle }
    }
    pub fn id(&self) -> &str {
        self.handle.id()
    }
    pub fn set_approval_mode(&self, mode: ApprovalMode) -> Result<()> {
        match mode {
            ApprovalMode::Manual => self.runtime.block_on(self.handle.use_manual_approval())?,
            ApprovalMode::Automatic => self
                .runtime
                .block_on(self.handle.use_automatic_approval())?,
        }
        Ok(())
    }
    pub fn pending_requests(&self) -> Result<Vec<PendingConsent>> {
        Ok(self.runtime.block_on(self.handle.pending_requests())?)
    }
    pub fn wait_for_request(&self, kind: ConsentKind) -> Result<PendingConsent> {
        Ok(self.runtime.block_on(self.handle.wait_for_consent(kind))?)
    }
    pub fn wait_for_request_approving_selection(
        &self,
        kind: ConsentKind,
    ) -> Result<PendingConsent> {
        Ok(self
            .runtime
            .block_on(self.handle.wait_for_consent_approving_selection(kind))?)
    }
    pub fn wait_for_any_request_approving_selection(
        &self,
        kinds: &[ConsentKind],
    ) -> Result<PendingConsent> {
        Ok(self
            .runtime
            .block_on(self.handle.wait_for_any_consent_approving_selection(kinds))?)
    }
    pub fn approve(&self, request: u64) -> Result<()> {
        Ok(self.runtime.block_on(self.handle.approve(request))?)
    }
    /// Erases credentials and resets approval mode to Automatic.
    pub fn reset(&self, profile: AuthenticatorProfile) -> Result<()> {
        Ok(self.runtime.block_on(self.handle.configure(profile))?)
    }
    pub fn info(&self) -> Result<AuthenticatorInfo> {
        Ok(self.runtime.block_on(self.handle.info())?)
    }
    pub fn request_count(&self) -> Result<u64> {
        Ok(self.info()?.consent_requests)
    }
    pub fn wait_for_no_pending_requests(&self) -> Result<()> {
        Ok(self
            .runtime
            .block_on(self.handle.wait_for_no_pending_requests())?)
    }
}

impl Drop for AuthenticatorControl {
    fn drop(&mut self) {
        if let Err(error) = self.runtime.block_on(self.handle.delete()) {
            eprintln!("Failed to delete authenticator {}: {error}", self.id());
        }
    }
}
