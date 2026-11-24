use crate::authenticator_server::client::{
    AuthenticatorInfo, AuthenticatorProfile, AuthenticatorServerClient, AuthenticatorServerError,
    ConsentKind, HybridAdvertisement, PendingConsent,
};
use std::time::Duration;

/// A retained virtual key, including while unplugged; requests are scoped to its ID.
#[derive(Clone, Debug)]
pub struct AuthenticatorHandle {
    server: AuthenticatorServerClient,
    id: String,
}
impl AuthenticatorHandle {
    pub(crate) fn new(server: AuthenticatorServerClient, id: String) -> Self {
        Self { server, id }
    }
    pub fn id(&self) -> &str {
        &self.id
    }

    pub async fn info(&self) -> Result<AuthenticatorInfo, AuthenticatorServerError> {
        Ok(serde_json::from_slice(
            &self.request("GET", "", &[]).await?,
        )?)
    }
    pub async fn attach(&self) -> Result<AuthenticatorInfo, AuthenticatorServerError> {
        Ok(serde_json::from_slice(
            &self.request("POST", "/attach", &[]).await?,
        )?)
    }
    pub async fn detach(&self) -> Result<AuthenticatorInfo, AuthenticatorServerError> {
        Ok(serde_json::from_slice(
            &self.request("POST", "/detach", &[]).await?,
        )?)
    }
    pub async fn delete(&self) -> Result<(), AuthenticatorServerError> {
        self.request("DELETE", "", &[]).await?;
        Ok(())
    }
    /// Configuration resets stored credentials and approval to automatic.
    pub async fn configure(
        &self,
        profile: AuthenticatorProfile,
    ) -> Result<(), AuthenticatorServerError> {
        self.request("POST", &format!("/configuration/{}", profile.path()), &[])
            .await?;
        Ok(())
    }
    pub async fn use_manual_approval(&self) -> Result<(), AuthenticatorServerError> {
        self.request("POST", "/mode/manual", &[]).await?;
        Ok(())
    }
    pub async fn use_automatic_approval(&self) -> Result<(), AuthenticatorServerError> {
        self.request("POST", "/mode/automatic", &[]).await?;
        Ok(())
    }
    pub async fn pending_requests(&self) -> Result<Vec<PendingConsent>, AuthenticatorServerError> {
        Ok(serde_json::from_slice(
            &self.request("GET", "/requests/pending", &[]).await?,
        )?)
    }
    pub async fn approve(&self, request_id: u64) -> Result<(), AuthenticatorServerError> {
        self.request("POST", &format!("/requests/{request_id}/approve"), &[])
            .await?;
        Ok(())
    }
    pub async fn scan_hybrid_qr(
        &self,
        qr_uri: &str,
    ) -> Result<HybridAdvertisement, AuthenticatorServerError> {
        Ok(serde_json::from_slice(
            &self
                .request("POST", "/hybrid/scan", qr_uri.as_bytes())
                .await?,
        )?)
    }
    pub async fn wait_for_consent(
        &self,
        kind: ConsentKind,
    ) -> Result<PendingConsent, AuthenticatorServerError> {
        self.wait_for_any_consent(&[kind]).await
    }
    pub async fn wait_for_pending_consent(
        &self,
    ) -> Result<PendingConsent, AuthenticatorServerError> {
        self.wait_for_any_consent(&[]).await
    }
    /// Empty `kinds` accepts any consent request.
    pub async fn wait_for_any_consent(
        &self,
        kinds: &[ConsentKind],
    ) -> Result<PendingConsent, AuthenticatorServerError> {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if let Some(request) = self
                    .pending_requests()
                    .await?
                    .into_iter()
                    .find(|request| kinds.is_empty() || kinds.contains(&request.kind))
                {
                    return Ok(request);
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await
        .map_err(|_| AuthenticatorServerError::TimedOut)?
    }
    /// Approve the selection touch and wait for the approval request for the original operation.
    pub async fn wait_for_consent_approving_selection(
        &self,
        kind: ConsentKind,
    ) -> Result<PendingConsent, AuthenticatorServerError> {
        self.wait_for_any_consent_approving_selection(&[kind]).await
    }
    pub async fn wait_for_any_consent_approving_selection(
        &self,
        kinds: &[ConsentKind],
    ) -> Result<PendingConsent, AuthenticatorServerError> {
        let mut accepted = kinds.to_vec();
        accepted.push(ConsentKind::Selection);
        loop {
            let request = self.wait_for_any_consent(&accepted).await?;
            if request.kind != ConsentKind::Selection {
                return Ok(request);
            }
            self.approve(request.id).await?;
        }
    }

    pub async fn wait_for_no_pending_requests(&self) -> Result<(), AuthenticatorServerError> {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !self.pending_requests().await?.is_empty() {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
            Ok(())
        })
        .await
        .map_err(|_| AuthenticatorServerError::TimedOut)?
    }
    async fn request(
        &self,
        method: &str,
        path: &str,
        body: &[u8],
    ) -> Result<Vec<u8>, AuthenticatorServerError> {
        self.server
            .request(method, &format!("/authenticators/{}{path}", self.id), body)
            .await
    }
}
