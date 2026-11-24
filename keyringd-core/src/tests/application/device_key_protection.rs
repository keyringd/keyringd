use crate::application::KeyringdApplication;
use crate::application::PasskeyChoice;
use crate::application::ProfileSetupRequest;
use crate::model::device_key::DeviceKeyAlgorithm;
use crate::model::device_key::{DevicePrivateKeyHandle, DevicePublicKey, LocalAuthentication};
use crate::service::DeviceKeyMaterial;
use crate::service::DeviceKeyProtectionChange;
use crate::service::DeviceKeyProtectionChangeResult;
use crate::test_support::{
    MemoryProfileRepository, MockDeviceKeyService, MockDeviceKeyServiceError, TestFidoMode,
    TestPasskeyService, test_device_public_key,
};
use p256::elliptic_curve::sec1::ToSec1Point;
use std::sync::Arc;

fn updated_key(public_key: DevicePublicKey) -> DeviceKeyMaterial {
    DeviceKeyMaterial {
        handle: DevicePrivateKeyHandle::new(b"new-key".to_vec()),
        public_key,
    }
}

fn different_public_key() -> DevicePublicKey {
    let point = p256::SecretKey::from_slice(&[2; 32])
        .unwrap()
        .public_key()
        .to_sec1_point(false);
    DevicePublicKey::from_ecdsa_p256(point.as_bytes()).unwrap()
}

fn keys(change: DeviceKeyProtectionChangeResult) -> MockDeviceKeyService {
    let mut keys = MockDeviceKeyService::new();
    keys.expect_generate_key().times(1).returning(|_, request| {
        Ok(DeviceKeyMaterial {
            handle: DevicePrivateKeyHandle::new(b"old-key".to_vec()),
            public_key: test_device_public_key(request.algorithm),
        })
    });
    keys.expect_change_key_protection()
        .times(1)
        .return_once(move |_, _| Ok(change));
    keys
}

async fn application(
    keys: MockDeviceKeyService,
    store: Arc<MemoryProfileRepository>,
    fido: Arc<TestPasskeyService>,
) -> Arc<KeyringdApplication<MockDeviceKeyService, Arc<TestPasskeyService>>> {
    let app = KeyringdApplication::new(keys, fido, store).unwrap();
    app.setup_profile(
        &Default::default(),
        &Default::default(),
        ProfileSetupRequest::new("Test".into(), PasskeyChoice::CreateNew),
        |_| {},
        crate::test_support::reject_credential_selection,
    )
    .await
    .unwrap();
    app
}

keyringd_testkit::scenario! {
    group "KeyringdApplication::change_device_key_protection" {
        case "Cleans the replacement or old handle according to commit outcome when the public key is unchanged" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            verify "commit deletes the old handle rollback deletes the replacement and cleanup failure does not undo a commit" {
                for (save_fails, cleanup_fails) in [(false, false), (true, false), (false, true)] {
                    let store = Arc::new(MemoryProfileRepository::default());
                    let fido = Arc::new(TestPasskeyService::default());
                    let change = DeviceKeyProtectionChangeResult::replacement(
                        updated_key(test_device_public_key(DeviceKeyAlgorithm::EcdsaP256)),
                        DevicePrivateKeyHandle::new(b"old-key".to_vec()),
                    );
                    let mut keys = keys(change);
                    let expected_deleted: &'static [u8] = if save_fails { b"new-key" } else { b"old-key" };
                    let saved_store = store.clone();
                    keys.expect_delete_key()
                        .times(1)
                        .withf(move |_, handle| handle.as_ref() == expected_deleted)
                        .returning(move |_, _| {
                            // Deletion follows persistence on success and preserves the old record on failure.
                            let saved = saved_store.snapshot().unwrap();
                            assert_eq!(
                                saved.profiles[0].device_key().handle().as_ref(),
                                if save_fails { b"old-key" } else { b"new-key" }
                            );
                            if cleanup_fails {
                                Err(MockDeviceKeyServiceError)
                            } else {
                                Ok(())
                            }
                        });
                    let app = application(keys, store.clone(), fido.clone()).await;
                    let original = app.profiles().remove(0);
                    let requests = fido.requests();
                    store.set_save_failure(save_fails);
                    let result = app
                        .change_device_key_protection(&Default::default(), original.id(), DeviceKeyProtectionChange::Enable)
                        .await;
                    assert_eq!(result.is_err(), save_fails);
                    assert_eq!(
                        fido.requests(),
                        requests,
                        "unchanged public keys reuse the certificate"
                    );
                    let current = app.profile(original.id()).unwrap();
                    assert_eq!(
                        current.device_key().handle().as_ref(),
                        if save_fails { b"old-key" } else { b"new-key" }
                    );
                    assert_eq!(
                        current.device_key().local_authentication(),
                        if save_fails {
                            LocalAuthentication::NotRequired
                        } else {
                            LocalAuthentication::Required
                        }
                    );
                    assert_eq!(
                        current.device_key().certificate().to_key_blob(),
                        original.device_key().certificate().to_key_blob()
                    );
                }
            }
        }

        case "Reissues the certificate without deleting keys when the backend specifies no cleanup handles" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            verify "certificate renewal and the save outcome do not cause deletion without a designated cleanup handle" {
                for save_fails in [false, true] {
                    let store = Arc::new(MemoryProfileRepository::default());
                    let fido = Arc::new(TestPasskeyService::default());
                    let mut keys = keys(DeviceKeyProtectionChangeResult::updated(updated_key(
                        different_public_key(),
                    )));
                    keys.expect_delete_key().never();
                    let app = application(keys, store.clone(), fido.clone()).await;
                    let original = app.profiles().remove(0);
                    let requests = fido.requests();
                    store.set_save_failure(save_fails);
                    let result = app
                        .change_device_key_protection(&Default::default(), original.id(), DeviceKeyProtectionChange::Enable)
                        .await;
                    assert_eq!(result.is_err(), save_fails);
                    assert!(
                        fido.requests() > requests,
                        "changed public keys require a new certificate"
                    );
                    assert_eq!(
                        app.profile(original.id())
                            .unwrap()
                            .device_key()
                            .handle()
                            .as_ref(),
                        if save_fails { b"old-key" } else { b"new-key" }
                    );
                }
            }
        }

        case "Preserves the original profile and cleans the replacement when certificate preparation fails" {
            async: tokio;
            timeout: std::time::Duration::from_secs(10);
            verify "certificate issuance failure retains the original profile and deletes only the replacement" {
                {
                    let public_key = different_public_key();
                    let store = Arc::new(MemoryProfileRepository::default());
                    let fido = Arc::new(TestPasskeyService::default());
                    let change = DeviceKeyProtectionChangeResult::replacement(
                        updated_key(public_key),
                        DevicePrivateKeyHandle::new(b"old-key".to_vec()),
                    );
                    let mut keys = keys(change);
                    keys.expect_delete_key()
                        .times(1)
                        .withf(|_, handle| handle.as_ref() == b"new-key")
                        .returning(|_, _| Ok(()));
                    let app = application(keys, store.clone(), fido.clone()).await;
                    let original = app.profiles().remove(0);
                    fido.set_mode(TestFidoMode::Failure);
                    assert!(
                        app.change_device_key_protection(&Default::default(), original.id(), DeviceKeyProtectionChange::Enable)
                            .await
                            .is_err()
                    );
                    assert_eq!(
                        app.profile(original.id())
                            .unwrap()
                            .device_key()
                            .handle()
                            .as_ref(),
                        b"old-key"
                    );
                    assert_eq!(
                        store.snapshot().unwrap().profiles[0]
                            .device_key()
                            .handle()
                            .as_ref(),
                        b"old-key"
                    );
                }
            }
        }
    }
}
