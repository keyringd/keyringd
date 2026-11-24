use super::{PinUvAuthError, PinUvAuthProtocol, PinUvAuthToken, PinUvSharedSecret};

use keyringd_fido::CtapCommand;
use keyringd_fido::ctap::{
    AuthenticatorClientPinRequest, AuthenticatorClientPinResponse, ClientPinSubcommand, Pin,
    PinUvAuthPermissions,
};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::ctap::{CtapClient, CtapClientError};

fn request(
    protocol: PinUvAuthProtocol,
    subcommand: ClientPinSubcommand,
) -> AuthenticatorClientPinRequest {
    AuthenticatorClientPinRequest::new(subcommand).with_pin_uv_auth_protocol(protocol.version())
}

/// Borrow a CTAP client to acquire PIN settings and PIN/Internal UV authentication tokens.
pub(crate) struct PinUvAuth<'a> {
    pub(in crate::ctap) client: &'a mut CtapClient,
}

impl<'a> PinUvAuth<'a> {
    pub(in crate::ctap) fn new(client: &'a mut CtapClient) -> Self {
        Self { client }
    }

    pub(crate) async fn get_token_using_configured_pin(
        &mut self,
        protocol: PinUvAuthProtocol,
        pin: &Pin,
        permissions: PinUvAuthPermissions,
        relying_party_id: Option<String>,
        supports_permission_tokens: bool,
    ) -> Result<PinUvAuthToken, PinUvAuthError> {
        if supports_permission_tokens {
            self.get_pin_uv_auth_token_using_pin(protocol, pin, permissions, relying_party_id)
                .await
        } else {
            self.get_pin_token_using_pin(protocol, pin).await
        }
    }
    pub(in crate::ctap) async fn execute(
        &mut self,
        request: &AuthenticatorClientPinRequest,
    ) -> Result<AuthenticatorClientPinResponse, CtapClientError> {
        let request = request.to_cbor_bytes()?;
        let payload = self
            .client
            .execute(keyringd_fido::CtapRequest::new(
                CtapCommand::ClientPin,
                request,
            ))
            .await?;
        AuthenticatorClientPinResponse::from_cbor_bytes(&payload).map_err(Into::into)
    }

    pub(in crate::ctap) async fn execute_without_response_data(
        &mut self,
        request: &AuthenticatorClientPinRequest,
    ) -> Result<(), CtapClientError> {
        let request = request.to_cbor_bytes()?;
        let payload = self
            .client
            .execute(keyringd_fido::CtapRequest::new(
                CtapCommand::ClientPin,
                request,
            ))
            .await?;
        if !payload.is_empty() {
            AuthenticatorClientPinResponse::from_cbor_bytes(&payload)?;
        }
        Ok(())
    }

    pub(in crate::ctap) async fn establish_shared_secret(
        &mut self,
        protocol: PinUvAuthProtocol,
    ) -> Result<(keyringd_fido::ctap::PinUvKeyAgreementKey, PinUvSharedSecret), PinUvAuthError>
    {
        let response = self
            .execute(&request(protocol, ClientPinSubcommand::GetKeyAgreement))
            .await?;
        let peer_key = response
            .key_agreement()
            .ok_or(PinUvAuthError::MissingResponseField("keyAgreement"))?;
        PinUvSharedSecret::establish(protocol, peer_key).map_err(Into::into)
    }

    pub(crate) async fn get_pin_retries(
        &mut self,
        protocol: PinUvAuthProtocol,
    ) -> Result<u64, PinUvAuthError> {
        self.execute(&request(protocol, ClientPinSubcommand::GetPinRetries))
            .await?
            .pin_retries()
            .ok_or(PinUvAuthError::MissingResponseField("pinRetries"))
    }

    pub(crate) async fn get_uv_retries(
        &mut self,
        protocol: PinUvAuthProtocol,
    ) -> Result<u64, PinUvAuthError> {
        self.execute(&request(protocol, ClientPinSubcommand::GetUvRetries))
            .await?
            .uv_retries()
            .ok_or(PinUvAuthError::MissingResponseField("uvRetries"))
    }

    /// Configures the first Client PIN on an authenticator.
    pub(crate) async fn set_pin(
        &mut self,
        protocol: PinUvAuthProtocol,
        pin: &Pin,
    ) -> Result<(), PinUvAuthError> {
        let (platform_key, shared_secret) = self.establish_shared_secret(protocol).await?;
        let pin = pin.expose_bytes();
        let mut padded_pin = Zeroizing::new(vec![0; 64]);
        padded_pin[..pin.len()].copy_from_slice(pin);
        let encrypted_pin = shared_secret.encrypt(&padded_pin)?;
        let authentication = shared_secret.authenticate(&encrypted_pin)?;
        let request = AuthenticatorClientPinRequest::new(ClientPinSubcommand::SetPin)
            .with_pin_uv_auth_protocol(protocol.version())
            .with_key_agreement(platform_key)
            .with_new_pin_enc(encrypted_pin)
            .with_pin_uv_auth_param(authentication);
        self.execute_without_response_data(&request).await?;
        Ok(())
    }

    /// Obtains an operation-scoped token from a PIN-configured authenticator.
    /// Verify CTAP configuration requirements and Unicode NFC normalization when building `Pin`.
    pub(crate) async fn get_pin_uv_auth_token_using_pin(
        &mut self,
        protocol: PinUvAuthProtocol,
        pin: &Pin,
        permissions: PinUvAuthPermissions,
        relying_party_id: Option<String>,
    ) -> Result<PinUvAuthToken, PinUvAuthError> {
        self.get_token_using_pin(protocol, pin, Some((permissions, relying_party_id)))
            .await
    }

    pub(crate) async fn get_pin_uv_auth_token_using_uv(
        &mut self,
        protocol: PinUvAuthProtocol,
        permissions: PinUvAuthPermissions,
        relying_party_id: Option<String>,
    ) -> Result<PinUvAuthToken, PinUvAuthError> {
        let (platform_key, shared_secret) = self.establish_shared_secret(protocol).await?;
        let mut request = AuthenticatorClientPinRequest::new(
            ClientPinSubcommand::GetPinUvAuthTokenUsingUvWithPermissions,
        )
        .with_pin_uv_auth_protocol(protocol.version())
        .with_key_agreement(platform_key)
        .with_permissions(permissions);
        if let Some(value) = relying_party_id {
            request = request.with_relying_party_id(value);
        }
        let token_response = self.execute(&request).await?;
        let encrypted_token = token_response
            .pin_uv_auth_token()
            .map(<[_]>::to_vec)
            .ok_or(PinUvAuthError::MissingResponseField("pinUvAuthToken"))?;
        shared_secret
            .decrypt_token(&encrypted_token)
            .map_err(Into::into)
    }

    pub(crate) async fn get_pin_token_using_pin(
        &mut self,
        protocol: PinUvAuthProtocol,
        pin: &Pin,
    ) -> Result<PinUvAuthToken, PinUvAuthError> {
        self.get_token_using_pin(protocol, pin, None).await
    }

    pub(in crate::ctap) async fn get_token_using_pin(
        &mut self,
        protocol: PinUvAuthProtocol,
        pin: &Pin,
        permission: Option<(PinUvAuthPermissions, Option<String>)>,
    ) -> Result<PinUvAuthToken, PinUvAuthError> {
        let (platform_key, shared_secret) = self.establish_shared_secret(protocol).await?;
        let hash = Sha256::digest(pin.expose_bytes());
        let pin_hash = Zeroizing::new(hash[..16].to_vec());
        let encrypted_pin_hash = shared_secret.encrypt(&pin_hash)?;
        let (subcommand, permissions, relying_party_id) = match permission {
            Some((permissions, relying_party_id)) => (
                ClientPinSubcommand::GetPinUvAuthTokenUsingPinWithPermissions,
                Some(permissions),
                relying_party_id,
            ),
            None => (ClientPinSubcommand::GetPinToken, None, None),
        };
        let mut request = AuthenticatorClientPinRequest::new(subcommand)
            .with_pin_uv_auth_protocol(protocol.version())
            .with_key_agreement(platform_key)
            .with_pin_hash_enc(encrypted_pin_hash.to_vec());
        if let Some(value) = permissions {
            request = request.with_permissions(value);
        }
        if let Some(value) = relying_party_id {
            request = request.with_relying_party_id(value);
        }
        let token_response = self.execute(&request).await?;
        let encrypted_token = token_response
            .pin_uv_auth_token()
            .map(<[_]>::to_vec)
            .ok_or(PinUvAuthError::MissingResponseField("pinUvAuthToken"))?;
        shared_secret
            .decrypt_token(&encrypted_token)
            .map_err(Into::into)
    }
}
