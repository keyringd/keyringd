use keyringd_fido::webauthn::LargeBlobOutput;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use p256::ecdsa::signature::hazmat::PrehashSigner;
use p256::ecdsa::{SigningKey, VerifyingKey};
use p256::elliptic_curve::{Generate, sec1::ToSec1Point};
use sha2::{Digest, Sha256};

use keyringd_fido::ctap::AuthenticatorGetAssertionResponse;
use keyringd_fido::webauthn::{
    AttestationObject, AttestationStatement, AuthenticationExtensionsClientOutputs,
    AuthenticatorAttachment, AuthenticatorAttestationResponse, AuthenticatorData,
    AuthenticatorTransport, PublicKeyCredentialCreationOptions, PublicKeyCredentialDescriptor,
    PublicKeyCredentialHint, PublicKeyCredentialType, PublicKeyCredentialUserEntity,
};
use keyringd_fido::{CredentialPublicKey, CtapStatusCode};

use super::TestCredential;
use super::TestCredentialSnapshot;
use super::TestFidoError;
use super::TestFidoOperation;
use super::{TestFidoInteractionHandler, TestFidoMode};
use keyringd_core::service::PasskeyService;
use keyringd_fido::transport::hybrid::{HybridOperationHint, HybridQrPayload};
use keyringd_fido_client::{
    ClientDataProvider, FidoClientError, GetAssertionsResult, MakeCredentialResult,
};
use keyringd_fido_client::{HybridTransportUnavailableReason, OperationId, OperationStatus};
use tokio::sync::watch;

/// In-memory FIDO implementation for tests. Creates credentials and signs with
/// P-256; modes control response timing, errors and optional product UI events.
pub struct TestPasskeyService {
    large_blob_supported: bool,
    large_blobs: Mutex<HashMap<Vec<u8>, Vec<u8>>>,
    credentials: Mutex<HashMap<Vec<u8>, TestCredential>>,
    requested_credential_ids: Mutex<Vec<Vec<Vec<u8>>>>,
    counter: Mutex<u32>,
    requested_hints: Mutex<Vec<Vec<PublicKeyCredentialHint>>>,
    handler: Option<Arc<dyn TestFidoInteractionHandler>>,
    mode: watch::Sender<TestFidoMode>,
    sequence: AtomicU64,
    pending: Arc<AtomicUsize>,
    cancelled: AtomicUsize,
    cancellations: Arc<Mutex<HashMap<OperationId, watch::Sender<bool>>>>,
}

impl Default for TestPasskeyService {
    fn default() -> Self {
        Self {
            large_blob_supported: false,
            large_blobs: Mutex::new(HashMap::new()),
            credentials: Mutex::new(HashMap::new()),
            requested_credential_ids: Mutex::new(Vec::new()),
            counter: Mutex::new(0),
            requested_hints: Mutex::new(Vec::new()),
            handler: None,
            mode: watch::channel(TestFidoMode::Success).0,
            sequence: AtomicU64::new(0),
            pending: Arc::new(AtomicUsize::new(0)),
            cancelled: AtomicUsize::new(0),
            cancellations: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl TestPasskeyService {
    pub fn with_large_blob_support(mut self) -> Self {
        self.large_blob_supported = true;
        self
    }
    pub fn set_large_blob(&self, id: Vec<u8>, data: Vec<u8>) {
        self.large_blobs.lock().unwrap().insert(id, data);
    }

    pub fn cancel(&self, id: OperationId) -> bool {
        if let Some(cancel) = self.cancellations.lock().unwrap().get(&id) {
            cancel.send_replace(true);
            true
        } else {
            false
        }
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn export_credentials(&self) -> Vec<TestCredentialSnapshot> {
        self.credentials
            .lock()
            .unwrap()
            .iter()
            .map(|(id, credential)| TestCredentialSnapshot {
                id: id.clone(),
                signing_key: credential.signing_key.to_bytes().to_vec(),
                relying_party_id: credential.relying_party_id.clone(),
                user_handle: credential.user_handle.clone(),
                user_name: credential.user_name.clone(),
                user_display_name: credential.user_display_name.clone(),
            })
            .collect()
    }
    pub fn import_credentials(
        &self,
        snapshots: Vec<TestCredentialSnapshot>,
    ) -> Result<(), FidoClientError> {
        let credentials = snapshots
            .into_iter()
            .map(|snapshot| {
                let signing_key = SigningKey::from_slice(&snapshot.signing_key)
                    .map_err(FidoClientError::backend)?;
                Ok((
                    snapshot.id,
                    TestCredential {
                        signing_key,
                        relying_party_id: snapshot.relying_party_id,
                        user_handle: snapshot.user_handle,
                        user_name: snapshot.user_name,
                        user_display_name: snapshot.user_display_name,
                    },
                ))
            })
            .collect::<Result<HashMap<_, _>, FidoClientError>>()?;
        *self.credentials.lock().unwrap() = credentials;
        Ok(())
    }

    pub fn requested_hints(&self) -> Vec<Vec<PublicKeyCredentialHint>> {
        self.requested_hints.lock().unwrap().clone()
    }
    pub fn requested_credential_ids(&self) -> Vec<Vec<Vec<u8>>> {
        self.requested_credential_ids.lock().unwrap().clone()
    }
}

impl PasskeyService for TestPasskeyService {
    async fn make_credential(
        &self,
        options: PublicKeyCredentialCreationOptions,
        parameters: std::sync::Arc<dyn ClientDataProvider>,
    ) -> Result<MakeCredentialResult, FidoClientError> {
        self.requested_hints
            .lock()
            .unwrap()
            .push(options.hints().to_vec());
        let _operation = self
            .wait_for_response(HybridOperationHint::MakeCredential)
            .await?;
        let client_data = parameters
            .provide(options.challenge())
            .map_err(FidoClientError::backend)?;
        let relying_party_id = options.rp().id().map(str::to_owned).ok_or_else(|| {
            FidoClientError::UnsupportedRequirement("test RP ID is required".into())
        })?;
        let signing_key = SigningKey::generate();
        let verifying_key = VerifyingKey::from(&signing_key);
        let mut credential_id = vec![0; 32];
        getrandom::fill(&mut credential_id).map_err(FidoClientError::backend)?;
        let public_key: [u8; 65] = verifying_key
            .to_sec1_point(false)
            .as_bytes()
            .try_into()
            .expect("uncompressed P-256 key");
        self.credentials.lock().unwrap().insert(
            credential_id.clone(),
            TestCredential {
                signing_key,
                relying_party_id: relying_party_id.clone(),
                user_handle: options.user().id().to_vec(),
                user_name: options.user().name().map(str::to_owned),
                user_display_name: options.user().display_name().map(str::to_owned),
            },
        );

        let credential_public_key = CredentialPublicKey::from_es256_sec1_public_key(&public_key)
            .map_err(FidoClientError::backend)?;
        let mut raw_authenticator_data = Vec::new();
        raw_authenticator_data.extend_from_slice(&Sha256::digest(relying_party_id.as_bytes()));
        raw_authenticator_data.push(0x41);
        raw_authenticator_data.extend_from_slice(&0u32.to_be_bytes());
        raw_authenticator_data.extend_from_slice(&[0; 16]);
        raw_authenticator_data.extend_from_slice(&(credential_id.len() as u16).to_be_bytes());
        raw_authenticator_data.extend_from_slice(&credential_id);
        raw_authenticator_data.extend_from_slice(
            &credential_public_key
                .to_cbor_bytes()
                .map_err(FidoClientError::backend)?,
        );
        let authenticator_data = AuthenticatorData::from_bytes(&raw_authenticator_data)
            .map_err(FidoClientError::backend)?;
        let response = AuthenticatorAttestationResponse::from_attestation_object(
            client_data.as_bytes().to_vec(),
            AttestationObject::new(authenticator_data, AttestationStatement::None),
            vec![AuthenticatorTransport::usb()],
        )
        .map_err(FidoClientError::backend)?;
        Ok(MakeCredentialResult::new(
            credential_id,
            response,
            Some(AuthenticatorAttachment::CrossPlatform),
            if self.large_blob_supported
                && options.extensions().and_then(|e| e.large_blob()).is_some()
            {
                AuthenticationExtensionsClientOutputs::default()
                    .with_large_blob(LargeBlobOutput::Supported(true))
            } else {
                AuthenticationExtensionsClientOutputs::default()
            },
        ))
    }
    async fn get_assertions(
        &self,
        options: keyringd_fido::webauthn::PublicKeyCredentialRequestOptions,
        _usb_device_id: Option<keyringd_fido_client::usb::UsbHidDeviceId>,
        provider: std::sync::Arc<dyn ClientDataProvider>,
    ) -> Result<GetAssertionsResult, FidoClientError> {
        let client_data = provider
            .provide(options.challenge())
            .map_err(FidoClientError::backend)?;
        self.requested_hints
            .lock()
            .unwrap()
            .push(options.hints().to_vec());
        self.requested_credential_ids.lock().unwrap().push(
            options
                .allow_credentials()
                .iter()
                .map(|credential| credential.id().to_vec())
                .collect(),
        );
        let _operation = self
            .wait_for_response(HybridOperationHint::GetAssertion)
            .await?;
        let relying_party_id = options
            .relying_party_id()
            .map(str::to_owned)
            .ok_or_else(|| {
                FidoClientError::UnsupportedRequirement("test RP ID is required".into())
            })?;
        let credentials = self.credentials.lock().unwrap();
        let matches = credentials
            .iter()
            .filter(|(id, credential)| {
                credential.relying_party_id == relying_party_id
                    && (options.allow_credentials().is_empty()
                        || options
                            .allow_credentials()
                            .iter()
                            .any(|descriptor| descriptor.id() == id.as_slice()))
            })
            .collect::<Vec<_>>();
        if matches.is_empty() {
            return Err(FidoClientError::Ctap(CtapStatusCode::NoCredentials));
        }
        let mut assertions = Vec::with_capacity(matches.len());
        for (credential_id, credential) in matches {
            let current_counter = {
                let mut counter = self.counter.lock().unwrap();
                *counter += 1;
                *counter
            };
            let mut authenticator_data = Vec::with_capacity(37);
            authenticator_data.extend_from_slice(&Sha256::digest(relying_party_id.as_bytes()));
            authenticator_data.push(0x05);
            authenticator_data.extend_from_slice(&current_counter.to_be_bytes());
            let mut signed_data = authenticator_data.clone();
            signed_data.extend_from_slice(&Sha256::digest(client_data.as_bytes()));
            let digest = Sha256::digest(&signed_data);
            let (signature, _) = credential
                .signing_key
                .sign_prehash(&digest)
                .map_err(|error| FidoClientError::backend(TestFidoError::from(error)))?;
            let descriptor = PublicKeyCredentialDescriptor::new(
                PublicKeyCredentialType::PublicKey,
                credential_id.clone(),
            );
            let mut user = PublicKeyCredentialUserEntity::from_id(credential.user_handle.clone());
            if let Some(name) = &credential.user_name {
                user = user.with_name(name.clone());
            }
            if let Some(name) = &credential.user_display_name {
                user = user.with_display_name(name.clone());
            }
            assertions.push(
                AuthenticatorGetAssertionResponse::new(
                    Some(descriptor),
                    AuthenticatorData::from_bytes(&authenticator_data)
                        .map_err(FidoClientError::backend)?,
                    signature.to_der().to_bytes().to_vec(),
                )
                .with_user(user),
            );
        }
        let mut result = GetAssertionsResult::from_ctap(
            assertions,
            client_data,
            AuthenticatorTransport::usb(),
            None,
        )?;
        if self.large_blob_supported {
            let ids = result
                .responses()
                .iter()
                .map(|r| r.credential().id().to_vec())
                .collect::<Vec<_>>();
            for id in ids {
                let input = options.extensions().and_then(|e| e.large_blob());
                if let Some(input) = input {
                    input
                        .validate_authentication()
                        .map_err(|error| FidoClientError::Protocol(error.into()))?;
                }
                let output = match input.map(|input| (input.read(), input.write())) {
                    Some((Some(true), _)) => Some(LargeBlobOutput::Read(
                        self.large_blobs.lock().unwrap().get(&id).cloned(),
                    )),
                    Some((_, Some(data))) => {
                        self.set_large_blob(id.clone(), data.to_vec());
                        Some(LargeBlobOutput::Written(true))
                    }
                    _ => None,
                };
                if let Some(output) = output {
                    result = result.with_large_blob(&id, output);
                }
            }
        }
        Ok(result)
    }
}

impl TestPasskeyService {
    pub fn set_mode(&self, mode: TestFidoMode) {
        self.mode.send_replace(mode);
    }

    pub fn requests(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
    }

    pub fn pending(&self) -> usize {
        self.pending.load(Ordering::SeqCst)
    }

    pub fn cancelled(&self) -> usize {
        self.cancelled.load(Ordering::SeqCst)
    }

    pub fn with_interaction_handler(
        mut self,
        handler: Arc<dyn TestFidoInteractionHandler>,
    ) -> Self {
        self.handler = Some(handler);
        self
    }

    async fn wait_for_mode(
        mode: &mut watch::Receiver<TestFidoMode>,
    ) -> Result<(), FidoClientError> {
        loop {
            match *mode.borrow_and_update() {
                TestFidoMode::Success => return Ok(()),
                TestFidoMode::Failure => {
                    return Err(FidoClientError::backend(std::io::Error::other(
                        "injected FIDO failure",
                    )));
                }
                TestFidoMode::NoCredentials => {
                    return Err(FidoClientError::Ctap(CtapStatusCode::NoCredentials));
                }
                TestFidoMode::Hold
                | TestFidoMode::HybridHold
                | TestFidoMode::BluetoothDisabled
                | TestFidoMode::NoBluetoothAdapter => {}
            }
            mode.changed().await.map_err(FidoClientError::backend)?;
        }
    }

    fn hybrid_qr_uri(hint: HybridOperationHint) -> Result<String, FidoClientError> {
        let key = p256::SecretKey::generate();
        let public_key = key.public_key().to_sec1_point(true);
        let mut secret = [0u8; 16];
        getrandom::fill(&mut secret).map_err(FidoClientError::backend)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(FidoClientError::backend)?
            .as_secs();
        HybridQrPayload::new(
            public_key
                .as_bytes()
                .try_into()
                .expect("compressed P-256 key"),
            secret,
            2,
            hint,
        )
        .with_current_time(now)
        .to_uri()
        .map_err(FidoClientError::backend)
    }
    pub(super) async fn wait_for_response(
        &self,
        hint: HybridOperationHint,
    ) -> Result<Option<TestFidoOperation>, FidoClientError> {
        let id = OperationId::new(self.sequence.fetch_add(1, Ordering::SeqCst) + 1);
        let mut mode = self.mode.subscribe();
        let initial = *mode.borrow();
        if initial == TestFidoMode::Success {
            return Ok(None);
        }
        let (cancel, mut cancellation) = watch::channel(false);
        self.cancellations.lock().unwrap().insert(id, cancel);
        self.pending.fetch_add(1, Ordering::SeqCst);
        let guard = TestFidoOperation::new(
            id,
            self.handler.clone(),
            Arc::clone(&self.pending),
            Arc::clone(&self.cancellations),
        );
        if let Some(handler) = self.handler.as_ref() {
            handler
                .on_operation_status_changed(OperationStatus::OperationStarted { operation_id: id })
                .await
                .map_err(FidoClientError::backend)?;
            handler
                .on_operation_status_changed(OperationStatus::Processing {
                    operation_id: id,
                    transport: AuthenticatorTransport::usb(),
                })
                .await
                .map_err(FidoClientError::backend)?;
            if matches!(
                initial,
                TestFidoMode::HybridHold
                    | TestFidoMode::BluetoothDisabled
                    | TestFidoMode::NoBluetoothAdapter
            ) {
                let uri = Self::hybrid_qr_uri(hint)?;
                handler
                    .present_hybrid_qr(id, &uri)
                    .await
                    .map_err(FidoClientError::backend)?;
                let reason = match initial {
                    TestFidoMode::BluetoothDisabled => {
                        Some(HybridTransportUnavailableReason::BluetoothDisabled)
                    }
                    TestFidoMode::NoBluetoothAdapter => {
                        Some(HybridTransportUnavailableReason::NoBluetoothAdapter)
                    }
                    _ => None,
                };
                if let Some(reason) = reason {
                    handler
                        .on_operation_status_changed(OperationStatus::HybridTransportUnavailable {
                            operation_id: id,
                            reason,
                        })
                        .await
                        .map_err(FidoClientError::backend)?;
                }
            }
        }
        tokio::select! {
            biased;
            _ = async {
                loop {
                    if *cancellation.borrow_and_update() { break; }
                    if cancellation.changed().await.is_err() { break; }
                }
            } => {
                self.cancelled.fetch_add(1, Ordering::SeqCst);
                Err(FidoClientError::Cancelled)
            }
            result = Self::wait_for_mode(&mut mode) => { result?; Ok(Some(guard)) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::RecordingFidoHandler;
    use super::{CtapStatusCode, HybridOperationHint, TestFidoMode, TestPasskeyService};
    use keyringd_fido_client::FidoClientError;
    use std::sync::Arc;
    use std::time::Duration;

    async fn wait_pending(service: &TestPasskeyService) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while service.pending() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    keyringd_testkit::scenario! {
        group "TestPasskeyService::cancel" {
            case "Cancel works without an interaction handler and removes the registration" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                verify "remove the cancelled operation from the active registry without a UI handler" {
                    let service = Arc::new(TestPasskeyService::new());
                    service.set_mode(TestFidoMode::Hold);
                    let task = tokio::spawn({
                        let service = service.clone();
                        async move {
                            service
                                .wait_for_response(HybridOperationHint::MakeCredential)
                                .await
                        }
                    });
                    wait_pending(&service).await;
                    assert!(!service.cancel(keyringd_fido_client::OperationId::new(2)));
                    assert!(service.cancel(keyringd_fido_client::OperationId::new(1)));
                    let result = tokio::time::timeout(Duration::from_secs(1), task)
                        .await
                        .unwrap()
                        .unwrap();
                    assert!(matches!(result, Err(FidoClientError::Cancelled)));
                    assert_eq!(service.pending(), 0);
                    assert!(!service.cancel(keyringd_fido_client::OperationId::new(1)));
                }
            }

            case "Cancellation closes the operation and updates counts" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                verify "record cancellation and leave no active test operation" {
                    let handler = Arc::new(RecordingFidoHandler::default());
                    let service = Arc::new(TestPasskeyService::new().with_interaction_handler(handler.clone()));
                    service.set_mode(TestFidoMode::Hold);
                    let task = tokio::spawn({
                        let service = Arc::clone(&service);
                        async move {
                            service
                                .wait_for_response(HybridOperationHint::MakeCredential)
                                .await
                        }
                    });
                    wait_pending(&service).await;
                    assert!(service.cancel(keyringd_fido_client::OperationId::new(1)));
                    let error = task.await.unwrap().err().unwrap();
                    assert!(matches!(&error, FidoClientError::Cancelled));
                    assert_eq!(service.cancelled(), 1);
                    assert_eq!(service.pending(), 0);
                    tokio::time::timeout(Duration::from_secs(2), async {
                        while !handler.events.lock().unwrap().contains(&"backend_finished") {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await
                    .unwrap();
                    assert_eq!(
                        *handler.events.lock().unwrap(),
                        ["started", "processing", "backend_finished"]
                    );
                }
            }
        }
        group "TestPasskeyService::wait_for_response" {
            case "Response modes work without a ui handler" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                verify "return the configured response without requesting UI input" {
                    let service = Arc::new(TestPasskeyService::new());
                    service.set_mode(TestFidoMode::Hold);
                    let task = tokio::spawn({
                        let service = Arc::clone(&service);
                        async move {
                            service
                                .wait_for_response(HybridOperationHint::MakeCredential)
                                .await
                        }
                    });
                    wait_pending(&service).await;
                    assert!(!task.is_finished());
                    service.set_mode(TestFidoMode::Success);
                    drop(task.await.unwrap().unwrap());
                    assert_eq!(service.requests(), 1);
                    assert_eq!(service.pending(), 0);
                    service.set_mode(TestFidoMode::Failure);
                    let error = service
                        .wait_for_response(HybridOperationHint::GetAssertion)
                        .await
                        .err()
                        .unwrap();
                    assert!(matches!(&error, FidoClientError::Backend(_)));
                    service.set_mode(TestFidoMode::NoCredentials);
                    let error = service
                        .wait_for_response(HybridOperationHint::GetAssertion)
                        .await
                        .err()
                        .unwrap();
                    assert!(matches!(
                        &error,
                        FidoClientError::Ctap(CtapStatusCode::NoCredentials)
                    ));
                    assert_eq!(service.pending(), 0);
                }
            }

            case "Bluetooth modes present a qr and the requested reason" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                verify "forward the configured reason with each Bluetooth QR request" {
                    for (mode, reason) in [
                        (TestFidoMode::BluetoothDisabled, "bluetooth-disabled"),
                        (TestFidoMode::NoBluetoothAdapter, "no-adapter"),
                    ] {
                        let handler = Arc::new(RecordingFidoHandler::default());
                        let service = Arc::new(TestPasskeyService::new().with_interaction_handler(handler.clone()));
                        service.set_mode(mode);
                        let task = tokio::spawn({
                            let service = Arc::clone(&service);
                            async move {
                                service
                                    .wait_for_response(HybridOperationHint::GetAssertion)
                                    .await
                            }
                        });
                        wait_pending(&service).await;
                        let uri = handler.uris.lock().unwrap()[0].clone();
                        assert!(uri.starts_with("FIDO:/"));
                        assert!(uri[6..].bytes().all(|byte| byte.is_ascii_digit()));
                        assert!(handler.events.lock().unwrap().contains(&reason));
                        assert!(service.cancel(keyringd_fido_client::OperationId::new(1)));
                        assert!(matches!(
                            &task.await.unwrap().err().unwrap(),
                            FidoClientError::Cancelled
                        ));
                        assert_eq!(service.pending(), 0);
                    }
                }
            }

            case "Dropping a held request clears pending state" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                verify "release the pending operation when its request is dropped" {
                    let service = TestPasskeyService::new();
                    service.set_mode(TestFidoMode::Hold);
                    assert!(
                        tokio::time::timeout(
                            Duration::from_millis(10),
                            service.wait_for_response(HybridOperationHint::GetAssertion)
                        )
                        .await
                        .is_err()
                    );
                    assert_eq!(service.pending(), 0);
                    assert_eq!(service.cancelled(), 0);
                }
            }
        }
    }
}
