use super::{DeviceKeyFidoAuthenticator, FidoAssertionProvider};
use crate::{application::ApplicationError, service::DeviceKeyService};
use keyringd_fido::webauthn::PublicKeyCredentialRequestOptions;
use keyringd_fido_client::{ClientDataProvider, FidoClient, FidoClientError, GetAssertionResult};
use std::sync::Arc;

/// Connects SSH assertion acquisition to the generic client and the selected Device Key.
pub(crate) struct DeviceKeyAssertionProvider<'a, D: DeviceKeyService> {
    client: FidoClient<(), DeviceKeyFidoAuthenticator<'a, D>>,
}

impl<'a, D: DeviceKeyService> DeviceKeyAssertionProvider<'a, D> {
    pub(crate) fn new(authenticator: DeviceKeyFidoAuthenticator<'a, D>) -> Self {
        Self {
            client: FidoClient::new(()).with_internal_authenticator(authenticator),
        }
    }
}

impl<D: DeviceKeyService> FidoAssertionProvider for DeviceKeyAssertionProvider<'_, D> {
    async fn get_assertion(
        &mut self,
        mut options: PublicKeyCredentialRequestOptions,
        client_data_provider: Arc<dyn ClientDataProvider>,
    ) -> Result<GetAssertionResult, ApplicationError> {
        let authenticator = self.client.internal_authenticator();
        if options.relying_party_id().is_none() {
            options = options.with_relying_party_id(authenticator.rp_id());
        }
        if options.allow_credentials().is_empty() {
            options = options.with_allow_credentials(vec![authenticator.credential().clone()]);
        }
        if options.hints().is_empty() {
            options = options.with_hints(vec![
                keyringd_fido::webauthn::PublicKeyCredentialHint::client_device(),
            ]);
        }
        let result = self
            .client
            .get_assertions(
                options,
                keyringd_fido_client::GetAssertionsContext::new(client_data_provider.as_ref())
                    .with_user_presence(false),
            )
            .await
            .map_err(|error| match error {
                FidoClientError::Cancelled | FidoClientError::OsCancelled => {
                    ApplicationError::OperationCanceled
                }
                error => ApplicationError::DeviceKey(Box::new(error)),
            })?;
        result.select(None).map_err(ApplicationError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::device_key::LocalAuthentication;
    use crate::service::DeviceKeySignature;
    use crate::test_support::MockDeviceKeyService;
    use crate::test_support::ModelProfileFixture;
    use keyringd_fido_client::ClientData;
    use sha2::{Digest, Sha256};

    keyringd_testkit::scenario! {
        group "DeviceKeyAssertionProvider::get_assertion" {
            case "Uses the fixed authenticator without caller credential or RP selection" {
                async: tokio;
                verify "the selected authenticator supplies its bound credential internally" {
                    let profile = ModelProfileFixture::default().build();
                    let expected_handle = profile.device_key().handle().clone();
                    let mut service = MockDeviceKeyService::new();
                    service.expect_sign().times(1)
                        .withf(move |_, request| request.handle.as_ref() == expected_handle.as_ref() && request.profile_name == "Work")
                        .returning(|_, _| Ok(DeviceKeySignature { signature: vec![1; 64], local_authentication: LocalAuthentication::NotRequired }));
                    let authenticator = DeviceKeyFidoAuthenticator::new(&service, profile.device_key(), &(), "ssh:bound-rp", "Work");
                    let mut provider = DeviceKeyAssertionProvider::new(authenticator);
                    let options = PublicKeyCredentialRequestOptions::new(b"message".to_vec());
                    assert!(options.allow_credentials().is_empty());
                    assert!(options.relying_party_id().is_none());
                    let assertion = provider.get_assertion(options, Arc::new(|message: &[u8]| Ok(ClientData::from_bytes(message.to_vec())))).await.unwrap();
                    assert_eq!(assertion.response().authenticator_data().rp_id_hash().as_slice(), Sha256::digest(b"ssh:bound-rp").as_slice());
                    assert_eq!(assertion.response().authenticator_data().flags(), 0);
                    assert_eq!(assertion.client_data().as_bytes(), b"message");
                }
            }
        }
    }
}
