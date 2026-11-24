use keyringd_fido_client::backend::UsbHidBackend;
mod support;
use keyringd_fido::webauthn::*;
use keyringd_fido_client::usb::CtapHidTransport;
use keyringd_fido_client::*;
use keyringd_test_usbip::{
    authenticator_server::client::{
        AuthenticatorHandle, AuthenticatorProfile, AuthenticatorServerError,
    },
    authenticator_server::manager::{AuthenticatorServerManager, AuthenticatorServerOptions},
    usbip_hid_backend::UsbIpHidBackend,
};
use std::sync::Arc;
use std::time::Duration;
use support::test_interaction_handler::TestInteractionHandler;

async fn start_server() -> AuthenticatorServerManager {
    static JAR: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    let jar = JAR.get_or_init(|| {
        keyringd_test_usbip::authenticator_server::build::AuthenticatorServerBuild::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(2)
                .unwrap(),
        )
        .prepare()
        .unwrap()
    });
    let log_directory = std::env::var_os("KEYRINGD_TEST_REPORT_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .ancestors()
                .nth(2)
                .unwrap()
                .join("target/test-reports/usbip")
        });
    std::fs::create_dir_all(&log_directory).unwrap();
    let log = tempfile::Builder::new()
        .prefix("authenticator-")
        .suffix(".log")
        .tempfile_in(log_directory)
        .unwrap()
        .into_temp_path()
        .keep()
        .unwrap();
    AuthenticatorServerManager::start(AuthenticatorServerOptions::new(jar.clone(), log))
        .await
        .unwrap()
}

fn options() -> PublicKeyCredentialCreationOptions {
    PublicKeyCredentialCreationOptions::new(
        PublicKeyCredentialRpEntity::new("USB/IP test").with_id("usbip.example"),
        PublicKeyCredentialUserEntity::new("test", vec![1, 2, 3], "Test user"),
        vec![7; 32],
        vec![PublicKeyCredentialParameters::es256()],
    )
    .with_timeout(15_000)
    .with_authenticator_selection(
        AuthenticatorSelectionCriteria::default()
            .with_resident_key(ResidentKeyRequirement::Required)
            .with_user_verification(UserVerificationRequirement::Required),
    )
}
fn client_data() -> Arc<dyn ClientDataProvider> {
    Arc::new(|challenge: &[u8]| Ok(ClientData::from_bytes(challenge.to_vec())))
}

keyringd_testkit::scenario! {
    group "virtual authenticators" {
        case "Multiple exports have stable ids and independent ctap channels" {
            async: tokio(flavor = "multi_thread");
            ignored: "requires a JDK";
            verify "multiple exports have stable ids and independent ctap channels" {
                let server = start_server().await;
                let first = server
                    .client()
                    .create_authenticator(Default::default())
                    .await
                    .unwrap();
                let second = server
                    .client()
                    .create_authenticator(Default::default())
                    .await
                    .unwrap();
                let backend = UsbIpHidBackend::new([server.usb_address()]);
                assert_ne!(first.id(), second.id());
                let devices = backend.enumerate().await.unwrap();
                assert_eq!(devices.len(), 2);
                assert_ne!(devices[0].id, devices[1].id);
                assert_eq!(devices, backend.enumerate().await.unwrap());
                let (a, b) = tokio::join!(backend.open(&devices[0].id), backend.open(&devices[1].id));
                let mut a = CtapHidTransport::new(a.unwrap(), None);
                let mut b = CtapHidTransport::new(b.unwrap(), None);
                let (a, b) = tokio::join!(a.initialize(), b.initialize());
                assert!(a.is_ok(), "{a:?}");
                assert!(b.is_ok(), "{b:?}");
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "virtual authenticators" {
        case "Pin registration and assertion use the injected backend" {
            async: tokio(flavor = "multi_thread");
            ignored: "requires a JDK";
            verify "pin registration and assertion use the injected backend" {
                let provider = client_data();
                let server = start_server().await;
                let key = server
                    .client()
                    .create_authenticator(Default::default())
                    .await
                    .unwrap();
                key.configure(AuthenticatorProfile::UvUnconfiguredPinConfigured)
                    .await
                    .unwrap();
                let handler = Arc::new(TestInteractionHandler::default());
                let client = FidoClient::new(handler.clone()).with_usb_backend(keyringd_fido_client::backend::UsbBackend::Hid(std::sync::Arc::new(
                        UsbIpHidBackend::new([server.usb_address()]),
                    ))).with_hybrid_connector(keyringd_fido_client::backend::SystemHybridChannelConnector::default());
                let credential = client
                    .make_credential(options(), keyringd_fido_client::MakeCredentialContext::new(provider.as_ref()))
                    .await
                    .unwrap();
                let assertions = client
                    .get_assertions(PublicKeyCredentialRequestOptions::new(vec![8; 32])
                            .with_relying_party_id("usbip.example")
                            .with_timeout(15_000)
                            .with_allow_credentials(vec![PublicKeyCredentialDescriptor::new(
                                credential.credential().credential_type().clone(),
                                credential.credential().raw_id().to_vec(),
                            )])
                            .with_user_verification(UserVerificationRequirement::Required)
                            .with_hints(vec![
                                keyringd_fido::webauthn::PublicKeyCredentialHint::security_key(),
                            ]), keyringd_fido_client::GetAssertionsContext::new(provider.as_ref()))
                    .await
                    .unwrap();
                assert_eq!(assertions.responses().len(), 1);
                assert_eq!(
                    assertions.responses()[0].credential().id(),
                    credential.credential().raw_id()
                );
                assert_eq!(*handler.pins.lock().unwrap(), 2);
            }
        }
    }
}

async fn approve_second_until_complete<T>(
    first: &AuthenticatorHandle,
    second: &AuthenticatorHandle,
    operation: impl std::future::Future<Output = T>,
) -> T {
    tokio::pin!(operation);
    let mut both_waited = false;
    let result = loop {
        tokio::select! {
            result = &mut operation => break result,
            _ = tokio::time::sleep(Duration::from_millis(20)) => {
                let pending = second.pending_requests().await.unwrap();
                if !pending.is_empty() && (!first.pending_requests().await.unwrap().is_empty() || both_waited) {
                    both_waited = true;
                    second.approve(pending[0].id).await.unwrap();
                }
            }
        }
    };
    assert!(
        both_waited,
        "both keys must wait before selecting the second"
    );
    result
}

keyringd_testkit::scenario! {
    group "virtual authenticators" {
        case "Selecting the second key cancels the first and next operation works" {
            async: tokio(flavor = "multi_thread");
            ignored: "requires a JDK";
            verify "selecting the second key cancels the first and next operation works" {
                let provider = client_data();
                let server = start_server().await;
                let first = server
                    .client()
                    .create_authenticator(Default::default())
                    .await
                    .unwrap();
                let second = server
                    .client()
                    .create_authenticator(Default::default())
                    .await
                    .unwrap();
                first.use_manual_approval().await.unwrap();
                second.use_manual_approval().await.unwrap();
                let handler = Arc::new(TestInteractionHandler::default());
                let client = FidoClient::new(handler.clone()).with_usb_backend(keyringd_fido_client::backend::UsbBackend::Hid(std::sync::Arc::new(
                        UsbIpHidBackend::new([server.usb_address()]),
                    ))).with_hybrid_connector(keyringd_fido_client::backend::SystemHybridChannelConnector::default());
                let credential = approve_second_until_complete(
                    &first,
                    &second,
                    client.make_credential(options().with_timeout(20_000), keyringd_fido_client::MakeCredentialContext::new(provider.as_ref())),
                )
                .await
                .unwrap();
                tokio::time::sleep(Duration::from_millis(100)).await;
                assert!(
                    first.pending_requests().await.unwrap().is_empty(),
                    "losing key still waits for user consent"
                );
                let assertions = approve_second_until_complete(
                    &first,
                    &second,
                    client.get_assertions(PublicKeyCredentialRequestOptions::new(vec![9; 32])
                            .with_relying_party_id("usbip.example")
                            .with_timeout(20_000)
                            .with_allow_credentials(vec![PublicKeyCredentialDescriptor::new(
                                credential.credential().credential_type().clone(),
                                credential.credential().raw_id().to_vec(),
                            )])
                            .with_user_verification(UserVerificationRequirement::Required)
                            .with_hints(vec![
                                keyringd_fido::webauthn::PublicKeyCredentialHint::security_key(),
                            ]), keyringd_fido_client::GetAssertionsContext::new(provider.as_ref())),
                )
                .await
                .unwrap();
                assert_eq!(assertions.responses().len(), 1);
                assert_eq!(
                    assertions.responses()[0].credential().id(),
                    credential.credential().raw_id()
                );
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "virtual authenticators" {
        case "Cancelling user presence clears testbed consent and allows retry" {
            async: tokio(flavor = "multi_thread");
            ignored: "requires a JDK";
            verify "cancelling user presence clears testbed consent and allows retry" {
                let provider = client_data();
                let server = start_server().await;
                let key = server
                    .client()
                    .create_authenticator(Default::default())
                    .await
                    .unwrap();
                key.use_manual_approval().await.unwrap();
                let handler = Arc::new(TestInteractionHandler::default());
                let client = FidoClient::new(handler.clone()).with_usb_backend(keyringd_fido_client::backend::UsbBackend::Hid(std::sync::Arc::new(
                        UsbIpHidBackend::new([server.usb_address()]),
                    ))).with_hybrid_connector(keyringd_fido_client::backend::SystemHybridChannelConnector::default());
                let cancel = async {
                    key.wait_for_pending_consent().await.unwrap();
                    let id = handler
                        .events
                        .lock()
                        .unwrap()
                        .iter()
                        .find_map(|event| match event {
                            OperationStatus::OperationStarted { operation_id } => Some(*operation_id),
                            _ => None,
                        })
                        .unwrap();
                    assert!(client.cancel(id));
                };
                let (result, ()) = tokio::join!(client.make_credential(options(), keyringd_fido_client::MakeCredentialContext::new(provider.as_ref())), cancel);
                assert!(matches!(&result.unwrap_err(), FidoClientError::Cancelled));
                tokio::time::sleep(Duration::from_millis(100)).await;
                assert!(key.pending_requests().await.unwrap().is_empty());
                key.use_automatic_approval().await.unwrap();
                client
                    .make_credential(options(), keyringd_fido_client::MakeCredentialContext::new(provider.as_ref()))
                    .await
                    .unwrap();
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "virtual authenticators" {
        case "Operation futures fit on gui worker stacks" {
            verify "operation futures fit on gui worker stacks" {
                let client = FidoClient::new(TestInteractionHandler::default()).with_usb_backend(keyringd_fido_client::backend::UsbBackend::Hid(std::sync::Arc::new(
                        UsbIpHidBackend::new([]),
                    ))).with_hybrid_connector(keyringd_fido_client::backend::SystemHybridChannelConnector::default());
                let provider = client_data();
                let create = client.make_credential(options(), keyringd_fido_client::MakeCredentialContext::new(provider.as_ref()));
                let create_size = std::mem::size_of_val(&create);
                drop(create);
                let get = client.get_assertions(PublicKeyCredentialRequestOptions::new(vec![1; 32])
                        .with_relying_party_id("usbip.example")
                        .with_timeout(15_000)
                        .with_hints(vec![
                            keyringd_fido::webauthn::PublicKeyCredentialHint::security_key(),
                        ]), keyringd_fido_client::GetAssertionsContext::new(provider.as_ref()));
                let get_size = std::mem::size_of_val(&get);
                eprintln!("operation future sizes: create={create_size}, get={get_size}");
                assert!(create_size < 32 * 1024, "create future size: {create_size}");
                assert!(get_size < 32 * 1024, "get future size: {get_size}");
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "virtual authenticators" {
        case "Http hotplug changes discovery and disconnects only the removed key" {
            async: tokio(flavor = "multi_thread");
            ignored: "requires a JDK";
            verify "http hotplug changes discovery and disconnects only the removed key" {
                let server = start_server().await;
                let backend = UsbIpHidBackend::new([server.usb_address()]);
                assert!(backend.enumerate().await.unwrap().is_empty());
                let first = server
                    .client()
                    .create_authenticator(Default::default())
                    .await
                    .unwrap();
                let second = server
                    .client()
                    .create_authenticator(Default::default())
                    .await
                    .unwrap();
                let devices = backend.enumerate().await.unwrap();
                assert_eq!(devices.len(), 2);
                let mut first_io = backend.open(&devices[0].id).await.unwrap();
                let mut second_channel =
                    CtapHidTransport::new(backend.open(&devices[1].id).await.unwrap(), None);
                first.detach().await.unwrap();
                let remaining = backend.enumerate().await.unwrap();
                assert_eq!(remaining, vec![devices[1].clone()]);
                use keyringd_fido_client::usb::HidPacketIo;
                let disconnected = tokio::time::timeout(Duration::from_secs(2), first_io.read_packet()).await;
                assert!(disconnected.unwrap().is_err());
                second_channel.initialize().await.unwrap();
                assert!(backend.open(&devices[0].id).await.is_err());
                first.attach().await.unwrap();
                let reenumerated = backend.enumerate().await.unwrap();
                assert_eq!(reenumerated.len(), 2);
                assert!(devices.iter().all(|device| reenumerated.contains(device)));
                let mut reattached = CtapHidTransport::new(backend.open(&devices[0].id).await.unwrap(), None);
                reattached.initialize().await.unwrap();
                first.delete().await.unwrap();
                assert!(matches!(
                    first.info().await.unwrap_err(),
                    AuthenticatorServerError::Http { status: 404, .. }
                ));
                assert_eq!(backend.enumerate().await.unwrap(), remaining);
                // IDs are not reused after deleting a key.
                let replacement = server
                    .client()
                    .create_authenticator(Default::default())
                    .await
                    .unwrap();
                assert_ne!(replacement.id(), first.id());
                assert_ne!(replacement.id(), second.id());
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "virtual authenticators" {
        case "Replugging the same authenticator preserves credentials for assertion" {
            async: tokio(flavor = "multi_thread");
            ignored: "requires a JDK";
            verify "replugging the same authenticator preserves credentials for assertion" {
                let provider = client_data();
                let server = start_server().await;
                let key = server
                    .client()
                    .create_authenticator(Default::default())
                    .await
                    .unwrap();
                let backend = UsbIpHidBackend::new([server.usb_address()]);
                let client = FidoClient::new(TestInteractionHandler::default()).with_usb_backend(keyringd_fido_client::backend::UsbBackend::Hid(std::sync::Arc::new(backend.clone()))).with_hybrid_connector(keyringd_fido_client::backend::SystemHybridChannelConnector::default());
                let credential = client
                    .make_credential(options(), keyringd_fido_client::MakeCredentialContext::new(provider.as_ref()))
                    .await
                    .unwrap();
                key.detach().await.unwrap();
                assert!(backend.enumerate().await.unwrap().is_empty());
                let listed = server.client().authenticators().await.unwrap();
                assert!(!listed[0].connected);
                key.attach().await.unwrap();
                let assertions = client
                    .get_assertions(PublicKeyCredentialRequestOptions::new(vec![10; 32])
                            .with_relying_party_id("usbip.example")
                            .with_timeout(15_000)
                            .with_allow_credentials(vec![PublicKeyCredentialDescriptor::new(
                                credential.credential().credential_type().clone(),
                                credential.credential().raw_id().to_vec(),
                            )])
                            .with_user_verification(UserVerificationRequirement::Required)
                            .with_hints(vec![
                                keyringd_fido::webauthn::PublicKeyCredentialHint::security_key(),
                            ]), keyringd_fido_client::GetAssertionsContext::new(provider.as_ref()))
                    .await
                    .unwrap();
                assert_eq!(
                    assertions.responses()[0].credential().id(),
                    credential.credential().raw_id()
                );
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "virtual authenticators" {
        case "Unplugging during user presence clears consent and replug continues the operation" {
            async: tokio(flavor = "multi_thread");
            ignored: "requires a JDK";
            verify "unplugging during user presence clears consent and replug continues the operation" {
                let provider = client_data();
                let server = start_server().await;
                let key = server
                    .client()
                    .create_authenticator(Default::default())
                    .await
                    .unwrap();
                let client = FidoClient::new(TestInteractionHandler::default()).with_usb_backend(keyringd_fido_client::backend::UsbBackend::Hid(std::sync::Arc::new(
                        UsbIpHidBackend::new([server.usb_address()]),
                    ))).with_hybrid_connector(keyringd_fido_client::backend::SystemHybridChannelConnector::default());
                // Bind the session to USB; an unbound session would also wait for Hybrid candidates.
                let credential = client
                    .make_credential(options(), keyringd_fido_client::MakeCredentialContext::new(provider.as_ref()))
                    .await
                    .unwrap();
                let request = PublicKeyCredentialRequestOptions::new(vec![11; 32])
                    .with_relying_party_id("usbip.example")
                    .with_timeout(15_000)
                    .with_allow_credentials(vec![PublicKeyCredentialDescriptor::new(
                        credential.credential().credential_type().clone(),
                        credential.credential().raw_id().to_vec(),
                    )])
                    .with_user_verification(UserVerificationRequirement::Required);
                key.use_manual_approval().await.unwrap();
                let unplug = async {
                    key.wait_for_pending_consent().await.unwrap();
                    assert!(matches!(
                        key.configure(AuthenticatorProfile::UvConfiguredPinConfigured)
                            .await
                            .unwrap_err(),
                        AuthenticatorServerError::Http { status: 409, .. }
                    ));
                    key.detach().await.unwrap();
                    assert!(
                        key.pending_requests().await.unwrap().is_empty(),
                        "unplugged key still has a consent request"
                    );
                    key.use_automatic_approval().await.unwrap();
                    key.attach().await.unwrap();
                };
                let (result, ()) = tokio::join!(
                    client.get_assertions(request.clone().with_hints(vec![
                            keyringd_fido::webauthn::PublicKeyCredentialHint::security_key()
                        ]), keyringd_fido_client::GetAssertionsContext::new(provider.as_ref())),
                    unplug
                );
                // Before adopting a key the broker keeps discovering devices, so plugging
                // the same key back in resumes this operation rather than requiring a retry.
                let result = result.unwrap();
                assert_eq!(
                    result.responses()[0].credential().id(),
                    credential.credential().raw_id()
                );
            }
        }
    }
}
