#[path = "../src/test_support/mock_fido_interaction_handler.rs"]
mod mock_fido_interaction_handler;
#[path = "../src/test_support/mock_os_usb_backend.rs"]
mod mock_os_usb_backend;
#[path = "../src/test_support/mock_usb_hid_backend.rs"]
mod mock_usb_hid_backend;

use keyringd_fido::webauthn::*;
use keyringd_fido_client::*;
use mock_fido_interaction_handler::{MockFidoInteractionHandler, usb_only_handler};
use mock_os_usb_backend::MockOsUsbBackend;
use mock_usb_hid_backend::empty_usb_backend;
use std::sync::Arc;

async fn assert_backend_path(
    client: &FidoClient<MockFidoInteractionHandler>,
    check_error: impl Fn(&FidoClientError),
) {
    let data: Arc<dyn ClientDataProvider> =
        Arc::new(|challenge: &[u8]| Ok(ClientData::from_bytes(challenge.to_vec())));
    let creation = PublicKeyCredentialCreationOptions::new(
        PublicKeyCredentialRpEntity::new("Configuration test").with_id("configuration.example"),
        PublicKeyCredentialUserEntity::new("test", vec![1], "Test"),
        vec![1; 32],
        vec![PublicKeyCredentialParameters::es256()],
    )
    .with_timeout(100)
    .with_authenticator_selection(
        AuthenticatorSelectionCriteria::default()
            .with_resident_key(ResidentKeyRequirement::Required),
    );
    check_error(
        &client
            .make_credential(
                creation.with_hints(vec![
                    keyringd_fido::webauthn::PublicKeyCredentialHint::security_key(),
                ]),
                keyringd_fido_client::MakeCredentialContext::new((data.clone()).as_ref()),
            )
            .await
            .unwrap_err(),
    );
    check_error(
        &client
            .get_assertions(
                PublicKeyCredentialRequestOptions::new(vec![2; 32])
                    .with_timeout(100)
                    .with_relying_party_id("configuration.example")
                    .with_hints(vec![
                        keyringd_fido::webauthn::PublicKeyCredentialHint::security_key(),
                    ]),
                keyringd_fido_client::GetAssertionsContext::new((data).as_ref()),
            )
            .await
            .unwrap_err(),
    );
}

keyringd_testkit::scenario! {
    group "backend configuration" {
        case "Platform backend handles both operations" {
            async: tokio;
            verify "platform backend handles both operations" {
                let mut platform = MockOsUsbBackend::new();
                platform
                    .expect_make_credential()
                    .times(1)
                    .returning(|_, _, _, _| {
                        Box::pin(async {
                            Err(FidoClientError::backend(std::io::Error::other(
                                "selected platform backend",
                            )))
                        })
                    });
                platform
                    .expect_get_assertions()
                    .times(1)
                    .returning(|_, _, _, _| {
                        Box::pin(async {
                            Err(FidoClientError::backend(std::io::Error::other(
                                "selected platform backend",
                            )))
                        })
                    });
                let platform = Arc::new(platform);
                let client = FidoClient::new(usb_only_handler()).with_usb_backend(backend::UsbBackend::Os(platform.clone())).with_hybrid_connector(keyringd_fido_client::backend::SystemHybridChannelConnector::default());
                assert_backend_path(&client, |error| {
                    assert!(matches!(error, FidoClientError::Backend(_)))
                })
                .await;
                drop(client);
                Arc::try_unwrap(platform).ok().unwrap().checkpoint();
            }
        }
    }
}
keyringd_testkit::scenario! {
    group "backend configuration" {
        case "Empty hid backend times out for both operations" {
            async: tokio;
            verify "empty hid backend times out for both operations" {
                let client = FidoClient::new(usb_only_handler()).with_usb_backend(backend::UsbBackend::Hid(Arc::new(empty_usb_backend()))).with_hybrid_connector(keyringd_fido_client::backend::SystemHybridChannelConnector::default());
                assert_backend_path(&client, |error| {
                    assert!(matches!(error, FidoClientError::TimedOut))
                })
                .await;
            }
        }
    }
}
