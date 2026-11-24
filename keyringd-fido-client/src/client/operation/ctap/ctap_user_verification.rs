use crate::client::operation::operation_context::OperationContext;
use crate::{
    AuthenticatorCapabilities, AuthenticatorCapabilityError, CtapClient, CtapClientError,
    FidoClientError, InteractionId, NewPinInteraction, PinInteraction, PinUvAuthError,
    PinUvAuthProtocol, PinUvAuthToken, SelectedUserVerification,
};
use keyringd_fido::ctap::AuthenticatorGetInfoResponse;
use keyringd_fido::ctap::{
    AuthenticatorGetAssertionRequest, AuthenticatorMakeCredentialRequest, PinUvAuthPermissions,
};
use keyringd_fido::{AuthenticatorTransport, UserVerificationRequirement};

/// Choose a PIN/UV method from the authenticator's capabilities and this request, and prepare for user verification.
/// Determine which authentication method you can use when you receive an internal UV failure or an additional token request.
/// Responsible for setting PIN input/setting, internal UV, retry, token acquisition and authentication information for request.
/// We borrow communication clients, capabilities, and routes only for necessary periods, and will not be in charge of registration/certification or ending of the channel.
pub(crate) struct CtapUserVerification<'a> {
    client: &'a mut CtapClient,
    capabilities: &'a AuthenticatorCapabilities,
    transport: &'a AuthenticatorTransport,
    selected_user_verification: SelectedUserVerification,
}

impl<'a> CtapUserVerification<'a> {
    pub(crate) fn new(
        client: &'a mut CtapClient,
        capabilities: &'a AuthenticatorCapabilities,
        transport: &'a AuthenticatorTransport,
        requirement: &UserVerificationRequirement,
    ) -> Result<Self, AuthenticatorCapabilityError> {
        let selected_user_verification =
            select_user_verification(capabilities.info(), requirement)?;
        Ok(Self {
            client,
            capabilities,
            transport,
            selected_user_verification,
        })
    }

    pub(in crate::client::operation) fn check_requirements(
        capabilities: &AuthenticatorCapabilities,
        requirement: &UserVerificationRequirement,
    ) -> Result<(), AuthenticatorCapabilityError> {
        select_user_verification(capabilities.info(), requirement).map(|_| ())
    }

    async fn set_pin_and_obtain_token(
        &mut self,
        context: &OperationContext<'_>,
        protocol: PinUvAuthProtocol,
        minimum_length: u64,
        permissions: PinUvAuthPermissions,
        relying_party_id: Option<String>,
    ) -> Result<PinUvAuthToken, FidoClientError> {
        let mut interaction_sequence = 2;
        let mut rejected_by_authenticator = false;
        let pin = loop {
            let pin = context
                .request_new_pin(NewPinInteraction::new(
                    InteractionId::new(context.id, interaction_sequence),
                    minimum_length,
                    rejected_by_authenticator,
                ))
                .await?;
            interaction_sequence += 1;
            let Some(pin) = pin else {
                continue;
            };
            match self.client.pin_uv_auth().set_pin(protocol, &pin).await {
                Ok(()) => break pin,
                Err(PinUvAuthError::Client(CtapClientError::Status(
                    keyringd_fido::CtapStatusCode::PinPolicyViolation,
                ))) => rejected_by_authenticator = true,
                Err(error) => return Err(FidoClientError::from(error)),
            }
        };
        self.client
            .pin_uv_auth()
            .get_token_using_configured_pin(
                protocol,
                &pin,
                permissions,
                relying_party_id,
                self.capabilities.supports_permission_tokens(),
            )
            .await
            .map_err(FidoClientError::from)
    }
    async fn obtain_pin_uv_auth_token(
        &mut self,
        context: &OperationContext<'_>,
        protocol: PinUvAuthProtocol,
        permissions: PinUvAuthPermissions,
        relying_party_id: Option<String>,
        supports_permission_tokens: bool,
    ) -> Result<PinUvAuthToken, FidoClientError> {
        let mut interaction_sequence = 2;
        loop {
            let attempts_remaining = self
                .client
                .pin_uv_auth()
                .get_pin_retries(protocol)
                .await
                .map_err(FidoClientError::from)?;
            let pin = context
                .request_pin(PinInteraction::new(
                    InteractionId::new(context.id, interaction_sequence),
                    Some(attempts_remaining),
                ))
                .await?;
            interaction_sequence += 1;
            let Some(pin) = pin else {
                continue;
            };
            let token = self
                .client
                .pin_uv_auth()
                .get_token_using_configured_pin(
                    protocol,
                    &pin,
                    permissions,
                    relying_party_id.clone(),
                    supports_permission_tokens,
                )
                .await;
            match token {
                Ok(token) => return Ok(token),
                Err(PinUvAuthError::Client(CtapClientError::Status(
                    keyringd_fido::CtapStatusCode::PinInvalid,
                ))) => continue,
                Err(error) => return Err(FidoClientError::from(error)),
            }
        }
    }
    async fn obtain_builtin_uv_or_pin_token(
        &mut self,
        context: &OperationContext<'_>,
        protocol: PinUvAuthProtocol,
        permissions: PinUvAuthPermissions,
        relying_party_id: Option<String>,
    ) -> Result<PinUvAuthToken, FidoClientError> {
        context
            .handler
            .on_operation_status_changed(crate::OperationStatus::BuiltInUserVerification {
                operation_id: context.id,
                transport: self.transport.clone(),
            })
            .await?;
        for attempt in 0..self.capabilities.preferred_platform_uv_attempts() {
            let result = context
                .run(self.client.pin_uv_auth().get_pin_uv_auth_token_using_uv(
                    protocol,
                    permissions,
                    relying_party_id.clone(),
                ))
                .await?;
            match result {
                Ok(token) => return Ok(token),
                Err(PinUvAuthError::Client(CtapClientError::Status(
                    keyringd_fido::CtapStatusCode::UserVerificationInvalid,
                ))) => {
                    let retries = context
                        .run(self.client.pin_uv_auth().get_uv_retries(protocol))
                        .await?
                        .map_err(FidoClientError::from)?;
                    if retries > 0
                        && attempt + 1 < self.capabilities.preferred_platform_uv_attempts()
                    {
                        continue;
                    }
                    if let Some(token) = self
                        .obtain_additional_pin_uv_auth_token(context, permissions, relying_party_id)
                        .await?
                    {
                        return Ok(token);
                    }
                    return Err(FidoClientError::Ctap(
                        keyringd_fido::CtapStatusCode::UserVerificationInvalid,
                    ));
                }
                Err(PinUvAuthError::Client(CtapClientError::Status(status)))
                    if matches!(
                        status,
                        keyringd_fido::CtapStatusCode::UserVerificationBlocked
                            | keyringd_fido::CtapStatusCode::NotAllowed
                    ) =>
                {
                    if let Some(token) = self
                        .obtain_additional_pin_uv_auth_token(context, permissions, relying_party_id)
                        .await?
                    {
                        return Ok(token);
                    }
                    return Err(FidoClientError::Ctap(status));
                }
                Err(error) => return Err(FidoClientError::from(error)),
            }
        }
        unreachable!("preferredPlatformUvAttempts is validated as positive")
    }

    pub(crate) async fn apply_make_user_verification(
        &mut self,
        context: &OperationContext<'_>,
        request: &mut AuthenticatorMakeCredentialRequest,
    ) -> Result<(), FidoClientError> {
        if self.selected_user_verification == SelectedUserVerification::BuiltInUvOption {
            *request = request.clone().with_option("uv", true);
            context
                .handler
                .on_operation_status_changed(context.user_confirmation_status(self.transport, true))
                .await?;
        } else if let Some(token) = self
            .obtain_selected_pin_uv_auth_token(
                context,
                PinUvAuthPermissions::MAKE_CREDENTIAL,
                Some(
                    request
                        .relying_party()
                        .id()
                        .expect("CTAP makeCredential requires rp.id")
                        .to_owned(),
                ),
            )
            .await?
        {
            *request = request.clone().with_pin_uv_auth(
                token
                    .authenticate(request.client_data_hash().as_bytes())
                    .map_err(FidoClientError::protocol)?,
                token.protocol().version(),
            );
        }
        Ok(())
    }
    pub(in crate::client::operation) async fn apply_make_required_pin_uv_auth(
        &mut self,
        context: &OperationContext<'_>,
        request: &mut AuthenticatorMakeCredentialRequest,
    ) -> Result<bool, FidoClientError> {
        let Some(token) = self
            .obtain_additional_pin_uv_auth_token(
                context,
                PinUvAuthPermissions::MAKE_CREDENTIAL,
                Some(
                    request
                        .relying_party()
                        .id()
                        .expect("CTAP makeCredential requires rp.id")
                        .to_owned(),
                ),
            )
            .await?
        else {
            return Ok(false);
        };
        *request = request.clone().with_pin_uv_auth(
            token
                .authenticate(request.client_data_hash().as_bytes())
                .map_err(FidoClientError::protocol)?,
            token.protocol().version(),
        );
        Ok(true)
    }
    pub(crate) async fn apply_get_user_verification(
        &mut self,
        context: &OperationContext<'_>,
        request: &mut AuthenticatorGetAssertionRequest,
        write_large_blob: bool,
    ) -> Result<Option<PinUvAuthToken>, FidoClientError> {
        if self.selected_user_verification == SelectedUserVerification::BuiltInUvOption {
            *request = request.clone().with_option("uv", true);
            context
                .handler
                .on_operation_status_changed(context.user_confirmation_status(self.transport, true))
                .await?;
        } else if let Some(token) = self
            .obtain_selected_pin_uv_auth_token(
                context,
                if write_large_blob {
                    PinUvAuthPermissions::GET_ASSERTION
                        .union(PinUvAuthPermissions::LARGE_BLOB_WRITE)
                } else {
                    PinUvAuthPermissions::GET_ASSERTION
                },
                Some(request.rp_id().to_owned()),
            )
            .await?
        {
            *request = request.clone().with_pin_uv_auth(
                token
                    .authenticate(request.client_data_hash().as_bytes())
                    .map_err(FidoClientError::protocol)?,
                token.protocol().version(),
            );
            return Ok(Some(token));
        }
        Ok(None)
    }
    pub(in crate::client::operation) async fn apply_get_required_pin_uv_auth(
        &mut self,
        context: &OperationContext<'_>,
        request: &mut AuthenticatorGetAssertionRequest,
        write_large_blob: bool,
    ) -> Result<Option<PinUvAuthToken>, FidoClientError> {
        let Some(token) = self
            .obtain_additional_pin_uv_auth_token(
                context,
                if write_large_blob {
                    PinUvAuthPermissions::GET_ASSERTION
                        .union(PinUvAuthPermissions::LARGE_BLOB_WRITE)
                } else {
                    PinUvAuthPermissions::GET_ASSERTION
                },
                Some(request.rp_id().to_owned()),
            )
            .await?
        else {
            return Ok(None);
        };
        *request = request.clone().with_pin_uv_auth(
            token
                .authenticate(request.client_data_hash().as_bytes())
                .map_err(FidoClientError::protocol)?,
            token.protocol().version(),
        );
        Ok(Some(token))
    }
    async fn obtain_selected_pin_uv_auth_token(
        &mut self,
        context: &OperationContext<'_>,
        permissions: PinUvAuthPermissions,
        relying_party_id: Option<String>,
    ) -> Result<Option<PinUvAuthToken>, FidoClientError> {
        let token = match self.selected_user_verification {
            SelectedUserVerification::None | SelectedUserVerification::BuiltInUvOption => {
                return Ok(None);
            }
            SelectedUserVerification::BuiltInUvToken(protocol) => {
                self.obtain_builtin_uv_or_pin_token(
                    context,
                    protocol,
                    permissions,
                    relying_party_id,
                )
                .await?
            }
            SelectedUserVerification::ClientPin(protocol) => {
                context
                    .run(self.obtain_pin_uv_auth_token(
                        context,
                        protocol,
                        permissions,
                        relying_party_id,
                        self.capabilities.supports_permission_tokens(),
                    ))
                    .await??
            }
            SelectedUserVerification::ClientPinSetup {
                protocol,
                minimum_length,
            } => {
                context
                    .run(self.set_pin_and_obtain_token(
                        context,
                        protocol,
                        minimum_length,
                        permissions,
                        relying_party_id,
                    ))
                    .await??
            }
        };
        Ok(Some(token))
    }
    async fn obtain_additional_pin_uv_auth_token(
        &mut self,
        context: &OperationContext<'_>,
        permissions: PinUvAuthPermissions,
        relying_party_id: Option<String>,
    ) -> Result<Option<PinUvAuthToken>, FidoClientError> {
        match next_user_verification(self.capabilities.info(), self.selected_user_verification) {
            Some(SelectedUserVerification::BuiltInUvToken(protocol)) => {
                context
                    .handler
                    .on_operation_status_changed(crate::OperationStatus::BuiltInUserVerification {
                        operation_id: context.id,
                        transport: self.transport.clone(),
                    })
                    .await?;
                let token = context
                    .run(self.client.pin_uv_auth().get_pin_uv_auth_token_using_uv(
                        protocol,
                        permissions,
                        relying_party_id,
                    ))
                    .await?
                    .map_err(FidoClientError::from)?;
                Ok(Some(token))
            }
            Some(SelectedUserVerification::ClientPin(protocol)) => {
                let token = context
                    .run(self.obtain_pin_uv_auth_token(
                        context,
                        protocol,
                        permissions,
                        relying_party_id,
                        self.capabilities.supports_permission_tokens(),
                    ))
                    .await??;
                Ok(Some(token))
            }
            _ => Ok(None),
        }
    }
}

fn next_user_verification(
    info: &AuthenticatorGetInfoResponse,
    selected: SelectedUserVerification,
) -> Option<SelectedUserVerification> {
    let option = |name: &str| info.options().and_then(|values| values.get(name)).copied();
    let protocol = match selected {
        SelectedUserVerification::None => {
            let protocol =
                select_pin_uv_auth_protocol(info.pin_uv_auth_protocols().unwrap_or(&[]))?;
            if option("uv") == Some(true) && option("pinUvAuthToken") == Some(true) {
                return Some(SelectedUserVerification::BuiltInUvToken(protocol));
            }
            protocol
        }
        SelectedUserVerification::BuiltInUvToken(protocol) => protocol,
        SelectedUserVerification::BuiltInUvOption
        | SelectedUserVerification::ClientPin(_)
        | SelectedUserVerification::ClientPinSetup { .. } => return None,
    };
    if info.force_pin_change() == Some(true)
        || option("clientPin") != Some(true)
        || option("noMcGaPermissionsWithClientPin") == Some(true)
    {
        return None;
    }
    Some(SelectedUserVerification::ClientPin(protocol))
}

fn select_pin_uv_auth_protocol(supported: &[u64]) -> Option<PinUvAuthProtocol> {
    const PLATFORM_PREFERENCE: [PinUvAuthProtocol; 2] =
        [PinUvAuthProtocol::Two, PinUvAuthProtocol::One];

    PLATFORM_PREFERENCE
        .into_iter()
        .find(|protocol| supported.contains(&protocol.version()))
}

pub(crate) fn select_user_verification(
    info: &AuthenticatorGetInfoResponse,
    requirement: &UserVerificationRequirement,
) -> Result<SelectedUserVerification, AuthenticatorCapabilityError> {
    let option = |name: &str| info.options().and_then(|values| values.get(name)).copied();
    let user_verification = if option("alwaysUv") == Some(true) {
        UserVerificationRequirement::Required
    } else {
        requirement.clone()
    };
    if user_verification == UserVerificationRequirement::Discouraged {
        return Ok(SelectedUserVerification::None);
    }
    if option("uv") == Some(true) {
        if option("pinUvAuthToken") == Some(true)
            && let Some(protocol) =
                select_pin_uv_auth_protocol(info.pin_uv_auth_protocols().unwrap_or(&[]))
        {
            return Ok(SelectedUserVerification::BuiltInUvToken(protocol));
        }
        return Ok(SelectedUserVerification::BuiltInUvOption);
    }
    if info.force_pin_change() == Some(true) {
        return Err(AuthenticatorCapabilityError::PinChangeRequired);
    }
    if option("clientPin").is_some() && option("noMcGaPermissionsWithClientPin") == Some(true) {
        return if user_verification == UserVerificationRequirement::Preferred {
            Ok(SelectedUserVerification::None)
        } else {
            Err(AuthenticatorCapabilityError::UserVerificationUnavailable)
        };
    }
    if option("clientPin") == Some(true) {
        let protocol = select_pin_uv_auth_protocol(info.pin_uv_auth_protocols().unwrap_or(&[]))
            .ok_or(AuthenticatorCapabilityError::PinUvAuthProtocolUnsupported)?;
        return Ok(SelectedUserVerification::ClientPin(protocol));
    }
    if user_verification == UserVerificationRequirement::Preferred {
        return Ok(SelectedUserVerification::None);
    }
    if option("clientPin") == Some(false) {
        // TODO: When `uv=false` and the authenticator advertises Bio Enrollment,
        // offer enrollment of its built-in user-verification method as an
        // alternative recovery path. Until that CTAP flow and its UI exist,
        // keyringd initializes ClientPIN with `setPIN`.
        // See CTAP 2.2 sections 6.5.5.7 and 6.7.
        let protocol = select_pin_uv_auth_protocol(info.pin_uv_auth_protocols().unwrap_or(&[]))
            .ok_or(AuthenticatorCapabilityError::PinUvAuthProtocolUnsupported)?;
        Ok(SelectedUserVerification::ClientPinSetup {
            protocol,
            minimum_length: info.min_pin_length().unwrap_or(4),
        })
    } else {
        Err(AuthenticatorCapabilityError::UserVerificationUnavailable)
    }
}
