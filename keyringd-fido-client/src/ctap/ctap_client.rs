use super::{CtapClientError, PinUvAuth};

use crate::CtapTransport;

use crate::util::wire_trace;

use keyringd_fido::ctap::{
    AuthenticatorGetAssertionRequest, AuthenticatorGetAssertionResponse,
    AuthenticatorGetInfoRequest, AuthenticatorGetInfoResponse, AuthenticatorMakeCredentialRequest,
    AuthenticatorMakeCredentialResponse, ClientDataHash, CtapVersion,
};

use keyringd_fido::{CtapCommand, CtapRequest, CtapStatusCode};

use keyringd_fido::webauthn::{PublicKeyCredentialDescriptor, PublicKeyCredentialType};
use rand_core::{OsRng, RngCore};
use std::time::Duration;

pub(crate) const CHANNEL_BUSY_RETRY_DELAY: Duration = Duration::from_millis(50);

pub(crate) const MAX_CHANNEL_BUSY_RETRIES: usize = 3;

/// Holds the connected CTAP Transport and encodes, sends, receives, and decodes commands.
/// The communication method is delegated to CtapTransport and does not depend on the specific type of USB/Hybrid.
pub struct CtapClient {
    transport: Box<dyn CtapTransport>,
}

impl CtapClient {
    pub fn new(transport: Box<dyn CtapTransport>) -> Self {
        Self { transport }
    }

    pub async fn get_info(&mut self) -> Result<AuthenticatorGetInfoResponse, CtapClientError> {
        let request = AuthenticatorGetInfoRequest::new();
        let payload = self
            .execute(CtapRequest::new(
                CtapCommand::GetInfo,
                request.to_bytes().to_vec(),
            ))
            .await?;
        AuthenticatorGetInfoResponse::from_cbor_bytes(&payload).map_err(Into::into)
    }

    pub(crate) fn pin_uv_auth(&mut self) -> PinUvAuth<'_> {
        PinUvAuth::new(self)
    }

    pub async fn make_credential(
        &mut self,
        request: &AuthenticatorMakeCredentialRequest,
    ) -> Result<AuthenticatorMakeCredentialResponse, CtapClientError> {
        let request = request.to_cbor_bytes()?;
        let payload = self
            .execute(CtapRequest::new(CtapCommand::MakeCredential, request))
            .await?;
        AuthenticatorMakeCredentialResponse::from_cbor_bytes(&payload).map_err(Into::into)
    }

    pub async fn get_assertion(
        &mut self,
        request: &AuthenticatorGetAssertionRequest,
        version: CtapVersion,
    ) -> Result<AuthenticatorGetAssertionResponse, CtapClientError> {
        let allow_list_present = request.allow_list().is_some();
        let request = request.to_cbor_bytes()?;
        let payload = self
            .execute(CtapRequest::new(CtapCommand::GetAssertion, request))
            .await?;
        let response = AuthenticatorGetAssertionResponse::from_cbor_bytes(&payload)?;
        if allow_list_present {
            // CTAP 2.0 §5.2 step 10 can return a count of one for an allow-listed match.
            // CTAP 2.1 and later omit this member whenever allowList is present.
            if let Some(count) = response.number_of_credentials()
                && !(version == CtapVersion::Fido20 && count == 1)
            {
                return Err(CtapClientError::NumberOfCredentialsWithAllowList);
            }
            if response.user_selected().is_some() {
                return Err(CtapClientError::UserSelectedWithAllowList);
            }
        }
        Ok(response)
    }

    pub async fn get_assertions(
        &mut self,
        request: &AuthenticatorGetAssertionRequest,
        version: CtapVersion,
    ) -> Result<Vec<AuthenticatorGetAssertionResponse>, CtapClientError> {
        let first = self.get_assertion(request, version).await?;
        let count = first.number_of_credentials().unwrap_or(1);
        let count =
            usize::try_from(count).map_err(|_| CtapClientError::CredentialCountTooLarge(count))?;
        let mut assertions = Vec::new();
        assertions
            .try_reserve(count)
            .map_err(|_| CtapClientError::CredentialCountTooLarge(count as u64))?;
        assertions.push(first);
        for _ in 1..count {
            assertions.push(self.get_next_assertion().await?);
        }
        Ok(assertions)
    }

    pub async fn get_next_assertion(
        &mut self,
    ) -> Result<AuthenticatorGetAssertionResponse, CtapClientError> {
        let payload = self
            .execute(CtapRequest::new(CtapCommand::GetNextAssertion, Vec::new()))
            .await?;
        let response = AuthenticatorGetAssertionResponse::from_cbor_bytes(&payload)?;
        if response.number_of_credentials().is_some() {
            return Err(CtapClientError::NumberOfCredentialsOnNextAssertion);
        }
        if response.user_selected().is_some() {
            return Err(CtapClientError::UserSelectedOnNextAssertion);
        }
        Ok(response)
    }

    pub async fn selection(&mut self, version: CtapVersion) -> Result<(), CtapClientError> {
        if version != CtapVersion::Fido20 {
            self.execute(CtapRequest::new(CtapCommand::Selection, Vec::new()))
                .await?;
            return Ok(());
        }

        let mut credential_id = vec![0; 32];
        let mut client_data_hash = [0; 32];
        OsRng.fill_bytes(&mut credential_id);
        OsRng.fill_bytes(&mut client_data_hash);
        let rp_id = format!(
            "keyringd-selection-{:016x}{:016x}.invalid",
            OsRng.next_u64(),
            OsRng.next_u64()
        );
        let request =
            AuthenticatorGetAssertionRequest::new(rp_id, ClientDataHash::new(client_data_hash))
                .with_allow_list(vec![PublicKeyCredentialDescriptor::new(
                    PublicKeyCredentialType::PublicKey,
                    credential_id,
                )])
                .with_option("up", true);

        // CTAP 2.0 §5.2 requires user presence before returning NO_CREDENTIALS.
        // A random RP and credential keep this probe separate from real assertions.
        match self
            .execute(CtapRequest::new(
                CtapCommand::GetAssertion,
                request.to_cbor_bytes()?,
            ))
            .await
        {
            Err(CtapClientError::Status(CtapStatusCode::NoCredentials)) => Ok(()),
            Err(error) => Err(error),
            Ok(_) => Err(CtapClientError::UnexpectedSelectionAssertion),
        }
    }

    pub async fn cancel(&mut self) -> Result<(), CtapClientError> {
        self.transport
            .cancel()
            .await
            .map_err(CtapClientError::Transport)
    }

    pub async fn close(&mut self) -> Result<(), CtapClientError> {
        self.transport
            .close()
            .await
            .map_err(CtapClientError::Transport)
    }

    pub(crate) async fn execute(
        &mut self,
        request: CtapRequest,
    ) -> Result<Vec<u8>, CtapClientError> {
        let mut channel_busy_retries = 0;
        loop {
            wire_trace::trace_bytes("ctap", "out", request.payload(), Some(request.command()));
            let response = self
                .transport
                .transmit(&request)
                .await
                .map_err(CtapClientError::Transport)?;
            wire_trace::trace_ctap_response(&response, request.command());
            let status = response.status();
            if status == CtapStatusCode::ChannelBusy
                && channel_busy_retries < MAX_CHANNEL_BUSY_RETRIES
            {
                channel_busy_retries += 1;
                tokio::time::sleep(CHANNEL_BUSY_RETRY_DELAY).await;
                continue;
            }
            if !status.is_success() {
                return Err(CtapClientError::Status(status));
            }
            return Ok(response.into_payload());
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::support::ctap_ctap_client::{assertion, request, success};

    use crate::ctap::test_support::recording_transport;

    use std::sync::{Arc, Mutex};

    use std::collections::VecDeque;

    use keyringd_fido::ctap::{AuthenticatorMakeCredentialRequest, ClientDataHash};
    use keyringd_fido::webauthn::{
        PublicKeyCredentialParameters, PublicKeyCredentialRpEntity, PublicKeyCredentialUserEntity,
    };

    use super::*;

    #[tokio::test]
    async fn allow_list_credential_count_depends_on_ctap_version() {
        let credential =
            PublicKeyCredentialDescriptor::new(PublicKeyCredentialType::PublicKey, vec![1]);
        let request = request().with_allow_list(vec![credential.clone()]);
        for version in [
            CtapVersion::Fido20,
            CtapVersion::Fido21Preview,
            CtapVersion::Fido21,
            CtapVersion::Fido23,
        ] {
            for count in [None, Some(1), Some(2)] {
                let response = assertion(count, 1);
                let commands = Arc::new(Mutex::new(Vec::new()));
                let mut client = CtapClient::new(Box::new(recording_transport(
                    VecDeque::from([success(&response, |value| value.to_cbor_bytes().unwrap())]),
                    commands.clone(),
                    Default::default(),
                )));
                let result = client.get_assertions(&request, version).await;
                match (version, count) {
                    (_, None) | (CtapVersion::Fido20, Some(1)) => {
                        let responses = result.unwrap();
                        assert_eq!(responses.len(), 1);
                        assert_eq!(responses[0].credential(), Some(&credential));
                        assert_eq!(responses[0].number_of_credentials(), count);
                    }
                    _ => assert!(matches!(
                        result,
                        Err(CtapClientError::NumberOfCredentialsWithAllowList)
                    )),
                }
                assert_eq!(*commands.lock().unwrap(), [CtapCommand::GetAssertion]);
            }
        }
    }

    #[tokio::test]
    async fn selection_probe_accepts_only_no_credentials() {
        use crate::test_support::MockCtapTransport;
        use keyringd_fido::CtapResponse;
        for status in [
            CtapStatusCode::NoCredentials,
            CtapStatusCode::Ok,
            CtapStatusCode::OperationDenied,
            CtapStatusCode::UserActionTimeout,
            CtapStatusCode::KeepaliveCancel,
            CtapStatusCode::InvalidCommand,
            CtapStatusCode::PinUvAuthTokenRequired,
        ] {
            let mut transport = MockCtapTransport::new();
            transport
                .expect_transmit()
                .times(1)
                .returning(move |request| {
                    assert_eq!(request.command(), CtapCommand::GetAssertion);
                    Box::pin(async move { Ok(CtapResponse::new(status, Vec::new())) })
                });
            let result = CtapClient::new(Box::new(transport))
                .selection(CtapVersion::Fido20)
                .await;
            match status {
                CtapStatusCode::NoCredentials => assert!(result.is_ok()),
                CtapStatusCode::Ok => assert!(matches!(
                    result,
                    Err(CtapClientError::UnexpectedSelectionAssertion)
                )),
                _ => assert!(
                    matches!(result, Err(CtapClientError::Status(actual)) if actual == status)
                ),
            }
        }
    }

    keyringd_testkit::scenario! {
        group "CtapClient::make_credential" {
            case "Rejects oversized user id before transmitting" {
                async: tokio;
                timeout: std::time::Duration::from_secs(60);
                verify "reject a user ID over the byte limit before invoking the transport" {
                    let request = AuthenticatorMakeCredentialRequest::new(
                        ClientDataHash::new([0; 32]),
                        PublicKeyCredentialRpEntity::from_rp_id("example.com"),
                        PublicKeyCredentialUserEntity::from_id(vec![0; 65]),
                        vec![PublicKeyCredentialParameters::es256()],
                    );
                    let commands = Arc::new(Mutex::new(Vec::new()));
                    let mut client = CtapClient::new(Box::new(recording_transport(
                        VecDeque::new(),
                        commands.clone(),
                        Default::default(),
                    )));
                    assert!(matches!(
                        client.make_credential(&request).await,
                        Err(CtapClientError::Codec(
                            keyringd_fido::ProtocolError::InvalidValue {
                                field: "user.id",
                                ..
                            }
                        )),
                    ));
                    assert!(commands.lock().unwrap().is_empty());
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "CtapClient::get_assertions" {
            case "Rejects a response reporting zero credentials" {
                why "A successful assertion containing a credential but claiming zero credentials is internally contradictory, so the client rejects it before enumeration logic can underflow."
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);
                given "Prepare CtapstatusCode, MockCtapTransport, and VecDeque" {
                    let first = assertion(None, 1);
                }
                when "Call first.to_cbor_bytes" {
                    let mut encoded = first.to_cbor_bytes().unwrap();
                }
                then "Verify that the results match CtapStatusCode::Ok and CtapClientError::Codec" {
                    assert!((0xa0..=0xb7).contains(&encoded[0]));
                    encoded[0] += 1;
                    encoded.extend_from_slice(&[0x05, 0x00]);
                    let mut malformed = vec![CtapStatusCode::Ok.as_byte()];
                    malformed.extend_from_slice(&encoded);
                    let commands = Arc::new(Mutex::new(Vec::new()));
                    let channel = recording_transport(
                        VecDeque::from([malformed]),
                        commands.clone(),
                        Default::default(),
                    );
                    let mut client = CtapClient::new(Box::new(channel));
                    assert!(matches!(
                        client.get_assertions(&request(), CtapVersion::Fido21).await,
                        Err(CtapClientError::Codec(_))
                    ));
                    assert_eq!(*commands.lock().unwrap(), vec![CtapCommand::GetAssertion]);
                }
            }
            case "Rejects an assertion count that cannot be allocated safely" {
                why "A peer-controlled credential count must not cause an unbounded or overflowing allocation even though CTAP does not prescribe a client memory limit."
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);
                given "an authenticator response claiming the maximum possible credential count" {
                    let response = assertion(Some(u64::MAX), 1);
                    let commands = Arc::new(Mutex::new(Vec::new()));
                    let mut client = CtapClient::new(Box::new(recording_transport(
                        VecDeque::from([success(&response, |value| value.to_cbor_bytes().unwrap())]),
                        commands.clone(),
                        Default::default(),
                    )));
                }
                when "the client prepares storage for all reported assertions" {
                    let result = client.get_assertions(&request(), CtapVersion::Fido21).await;
                }
                then "the client rejects the count without attempting an unbounded allocation" {
                    assert!(matches!(
                        result,
                        Err(CtapClientError::CredentialCountTooLarge(u64::MAX))
                    ));
                    assert_eq!(*commands.lock().unwrap(), vec![CtapCommand::GetAssertion]);
                }
            }
        }
    }
}
