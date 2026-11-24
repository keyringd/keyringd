use crate::application::PasskeyCredentialCandidate;
use crate::application::{ApplicationError, OperationRejectionReason};
use crate::model::passkey::CredentialId;

/// Test callback for scenarios where no credential selection is expected.
pub async fn reject_credential_selection(
    _: Vec<PasskeyCredentialCandidate>,
) -> Result<CredentialId, ApplicationError> {
    Err(OperationRejectionReason::PasskeySelectionRequired.into())
}
