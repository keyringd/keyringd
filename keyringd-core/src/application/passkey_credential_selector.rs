use crate::application::ApplicationError;
use crate::application::PasskeyCredentialCandidate;
use crate::model::passkey::CredentialId;

/// Selects one of the offered credentials, or reports cancellation or failure.
/// The selection future may be dropped when profile setup is canceled.
pub trait PasskeyCredentialSelector: Send {
    fn select_credential(
        &mut self,
        candidates: Vec<PasskeyCredentialCandidate>,
    ) -> impl Future<Output = Result<CredentialId, ApplicationError>> + Send;
}

impl<F, Fut> PasskeyCredentialSelector for F
where
    F: FnMut(Vec<PasskeyCredentialCandidate>) -> Fut + Send,
    Fut: Future<Output = Result<CredentialId, ApplicationError>> + Send,
{
    async fn select_credential(
        &mut self,
        candidates: Vec<PasskeyCredentialCandidate>,
    ) -> Result<CredentialId, ApplicationError> {
        self(candidates).await
    }
}
