use crate::{FidoClientError, OperationId};
use keyringd_fido::webauthn::{
    PublicKeyCredentialCreationOptions, PublicKeyCredentialRequestOptions,
};
use std::sync::{Arc, Mutex};

/// Records of input and start/cancellation received by the OS backend mock.
#[derive(Default)]
pub(crate) struct OsBackendCalls {
    pub(crate) active: Mutex<Option<OperationId>>,
    pub(crate) cancelled: Mutex<Vec<OperationId>>,
    pub(crate) make_credential_algorithms: Mutex<Option<Vec<i32>>>,
    pub(crate) make_credential_options: Mutex<Option<PublicKeyCredentialCreationOptions>>,
    pub(crate) make_credential_client_data: Mutex<Option<Vec<u8>>>,
    pub(crate) get_assertions_options: Mutex<Option<PublicKeyCredentialRequestOptions>>,
    pub(crate) get_assertions_client_data: Mutex<Option<Vec<u8>>>,
}

pub(crate) fn pending_os_backend(
    calls: Arc<OsBackendCalls>,
) -> Arc<crate::test_support::MockOsUsbBackend> {
    let mut backend = crate::test_support::MockOsUsbBackend::new();
    {
        let calls = calls.clone();
        backend.expect_make_credential().returning(
            move |operation_id, options, client_data, cancellation| {
                let calls = calls.clone();
                Box::pin(async move {
                    *calls.make_credential_algorithms.lock().unwrap() = Some(
                        options
                            .public_key_credential_parameters()
                            .iter()
                            .map(|parameter| parameter.algorithm().value())
                            .collect(),
                    );
                    *calls.make_credential_options.lock().unwrap() = Some(options);
                    *calls.make_credential_client_data.lock().unwrap() =
                        Some(client_data.as_bytes().to_vec());
                    *calls.active.lock().unwrap() = Some(operation_id);
                    cancellation.cancelled().await;
                    calls.cancelled.lock().unwrap().push(operation_id);
                    *calls.active.lock().unwrap() = None;
                    Err(FidoClientError::Cancelled)
                })
            },
        );
    }
    {
        let calls = calls.clone();
        backend.expect_get_assertions().returning(
            move |operation_id, options, client_data, cancellation| {
                let calls = calls.clone();
                Box::pin(async move {
                    *calls.get_assertions_options.lock().unwrap() = Some(options);
                    *calls.get_assertions_client_data.lock().unwrap() =
                        Some(client_data.as_bytes().to_vec());
                    *calls.active.lock().unwrap() = Some(operation_id);
                    cancellation.cancelled().await;
                    calls.cancelled.lock().unwrap().push(operation_id);
                    *calls.active.lock().unwrap() = None;
                    Err(FidoClientError::Cancelled)
                })
            },
        );
    }
    Arc::new(backend)
}
