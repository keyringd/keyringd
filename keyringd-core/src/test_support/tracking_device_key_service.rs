use keyringd_core::service::DeviceKeyMaterial;
use keyringd_core::service::DeviceKeyProtectionChangeResult;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use p256::elliptic_curve::sec1::ToSec1Point;

use super::{MockDeviceKeyServiceError, test_device_public_key};
use super::{OperationJournal, TestOperation};
use keyringd_core::model::device_key::DeviceKeyAlgorithm;
use keyringd_core::model::device_key::DevicePrivateKeyHandle;
use keyringd_core::model::device_key::DevicePublicKey;
use keyringd_core::service::DeviceKeyGenerationRequest;
use keyringd_core::service::DeviceKeyProtectionChangeRequest;
use keyringd_core::service::DeviceKeyService;

use super::TrackedDeviceKeyOperationContext;

#[derive(Clone, Default)]
pub(crate) struct TrackingDeviceKeyService {
    generated: Arc<AtomicUsize>,
    generated_handles: Arc<Mutex<Vec<DevicePrivateKeyHandle>>>,
    deleted: Arc<Mutex<Vec<DevicePrivateKeyHandle>>>,
    operation_contexts: Arc<Mutex<Vec<TrackedDeviceKeyOperationContext>>>,
    journal: Option<OperationJournal>,
}

impl TrackingDeviceKeyService {
    pub(crate) fn with_journal(journal: OperationJournal) -> Self {
        Self {
            journal: Some(journal),
            ..Self::default()
        }
    }

    pub(crate) fn generated_count(&self) -> usize {
        self.generated.load(Ordering::SeqCst)
    }

    pub(crate) fn generated_handles(&self) -> Vec<DevicePrivateKeyHandle> {
        self.generated_handles.lock().unwrap().clone()
    }

    pub(crate) fn deleted_handles(&self) -> Vec<DevicePrivateKeyHandle> {
        self.deleted.lock().unwrap().clone()
    }

    pub(crate) fn operation_contexts(&self) -> Vec<TrackedDeviceKeyOperationContext> {
        self.operation_contexts.lock().unwrap().clone()
    }

    fn record_context(&self, context: &TrackedDeviceKeyOperationContext) {
        self.operation_contexts.lock().unwrap().push(*context);
    }
}

impl DeviceKeyService for TrackingDeviceKeyService {
    type Error = MockDeviceKeyServiceError;
    type OperationContext = TrackedDeviceKeyOperationContext;
    async fn check_availability(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn generate_key(
        &self,
        context: &Self::OperationContext,
        request: DeviceKeyGenerationRequest<'_>,
    ) -> Result<DeviceKeyMaterial, Self::Error> {
        self.record_context(context);
        let sequence = self.generated.fetch_add(1, Ordering::SeqCst) + 1;
        let handle = DevicePrivateKeyHandle::new(format!("tracking-key-{sequence}").into_bytes());
        self.generated_handles.lock().unwrap().push(handle.clone());
        Ok(DeviceKeyMaterial {
            handle,
            public_key: tracked_device_public_key(request.algorithm, sequence),
        })
    }

    async fn change_key_protection(
        &self,
        context: &Self::OperationContext,
        request: DeviceKeyProtectionChangeRequest<'_>,
    ) -> Result<DeviceKeyProtectionChangeResult, Self::Error> {
        self.record_context(context);
        let mut handle = request.handle.as_ref().to_vec();
        handle.extend_from_slice(b"-updated-protection");
        Ok(DeviceKeyProtectionChangeResult::updated(
            DeviceKeyMaterial {
                handle: DevicePrivateKeyHandle::new(handle),
                public_key: test_device_public_key(request.algorithm),
            },
        ))
    }

    async fn sign(
        &self,
        context: &Self::OperationContext,
        _request: keyringd_core::service::DeviceKeySigningRequest<'_>,
    ) -> Result<keyringd_core::service::DeviceKeySignature, Self::Error> {
        self.record_context(context);
        let mut bytes = [0u8; 64];
        bytes[..32].fill(1);
        bytes[32..].fill(2);
        Ok(keyringd_core::service::DeviceKeySignature {
            signature: bytes.to_vec(),
            local_authentication:
                keyringd_core::model::device_key::LocalAuthentication::NotRequired,
        })
    }

    async fn delete_key(
        &self,
        context: &Self::OperationContext,
        handle: &DevicePrivateKeyHandle,
    ) -> Result<(), Self::Error> {
        self.record_context(context);
        if let Some(journal) = &self.journal {
            journal.record(TestOperation::DeviceKeyDeleted(handle.as_ref().to_vec()));
        }
        self.deleted.lock().unwrap().push(handle.clone());
        Ok(())
    }
}

fn tracked_device_public_key(algorithm: DeviceKeyAlgorithm, sequence: usize) -> DevicePublicKey {
    let scalar = u8::try_from(sequence).expect("test key sequence must fit in one byte");
    let point = p256::SecretKey::from_slice(&[scalar; 32])
        .expect("a non-zero repeated byte is a valid fixed test key")
        .public_key()
        .to_sec1_point(false);
    assert!(matches!(algorithm, DeviceKeyAlgorithm::EcdsaP256));
    DevicePublicKey::from_ecdsa_p256(point.as_bytes()).unwrap()
}
