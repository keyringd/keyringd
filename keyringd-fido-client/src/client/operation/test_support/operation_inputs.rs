use super::super::operation_context::OperationContext;

use crate::client::PreparedMakeCredential;

use std::collections::BTreeMap;

use std::time::{Duration, Instant};

use keyringd_fido::UserVerificationRequirement;

use keyringd_fido::ctap::AuthenticatorGetInfoResponse;

use keyringd_fido::webauthn::{
    AuthenticatorSelectionCriteria, PublicKeyCredentialCreationOptions,
    PublicKeyCredentialParameters, PublicKeyCredentialRpEntity, PublicKeyCredentialUserEntity,
};

use crate::*;

use keyringd_fido::ResidentKeyRequirement;

use crate::ClientData;

pub(crate) fn make_options(
    user_verification: UserVerificationRequirement,
    user_id: u8,
) -> PreparedMakeCredential {
    PreparedMakeCredential::new(
        PublicKeyCredentialCreationOptions::new(
            PublicKeyCredentialRpEntity::new("Keyringd").with_id("ssh:keyringd"),
            PublicKeyCredentialUserEntity::new("user", vec![user_id], "User"),
            vec![1],
            vec![PublicKeyCredentialParameters::es256()],
        )
        .with_hints(vec![
            keyringd_fido::webauthn::PublicKeyCredentialHint::hybrid(),
        ])
        .with_authenticator_selection(
            AuthenticatorSelectionCriteria::default()
                .with_resident_key(ResidentKeyRequirement::Required)
                .with_user_verification(user_verification),
        ),
        (std::sync::Arc::new(|_: &[u8]| Ok(ClientData::from_bytes(vec![1])))).as_ref(),
    )
    .unwrap()
}

pub(crate) fn test_context<H: FidoInteractionHandler>(
    handler: &H,
    id: OperationId,
) -> OperationContext<'_> {
    OperationContext {
        id,
        handler,
        deadline: Instant::now() + Duration::from_secs(5),
        cancellation: tokio_util::sync::CancellationToken::new(),
    }
}

pub(crate) fn model_capabilities() -> AuthenticatorCapabilities {
    AuthenticatorCapabilities::from_get_info(
        AuthenticatorGetInfoResponse::new(vec!["FIDO_2_1".to_owned()], [0; 16])
            .with_options(BTreeMap::from([("rk".to_owned(), true)]))
            .with_algorithms(vec![PublicKeyCredentialParameters::es256()])
            .with_max_message_size(1024),
    )
}
