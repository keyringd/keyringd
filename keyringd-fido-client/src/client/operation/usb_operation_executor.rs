use super::UsbAuthenticatorSelector;
use crate::backend::UsbBackend;
use crate::{
    FidoClientError, GetAssertionOperationContext, GetAssertionsResult,
    MakeCredentialOperationContext, MakeCredentialResult,
};

/// Candidate search for USB routes and the execution part of registration/authentication. Delegate processing to HID and OS WebAuthn backend.
/// Select one of the multiple HID candidates by touch and proceed with PIN/UV registration/authentication with just that key.
pub(super) struct UsbOperationExecutor {
    backend: UsbBackend,
}

impl UsbOperationExecutor {
    pub(super) fn new(backend: UsbBackend) -> Self {
        Self { backend }
    }
    pub(super) async fn make_credential(
        &self,
        mut context: MakeCredentialOperationContext<'_>,
    ) -> Result<MakeCredentialResult, FidoClientError> {
        let operation = context
            .operation
            .with_cancellation(context.operation.cancellation.clone());
        let stop = operation.cancellation.child_token();
        context.operation.cancellation = stop.clone();
        let execute = async {
            match &self.backend {
                UsbBackend::Hid(backend) => {
                    let authenticator = UsbAuthenticatorSelector::new(backend.as_ref())
                        .select(
                            &context.operation,
                            context.prepared.authenticator_requirements(),
                            crate::AuthenticatorRequest::MakeCredential(
                                context.prepared.ctap_request(),
                            ),
                            context.usb_device_id.as_ref(),
                        )
                        .await?;
                    authenticator.make_credential(&context).await
                }
                UsbBackend::Os(backend) => {
                    if context.usb_device_id.is_some() {
                        return Err(FidoClientError::NoAuthenticator);
                    }
                    let operation = &context.operation;
                    backend
                        .make_credential(
                            operation.id,
                            context
                                .prepared
                                .options
                                .clone()
                                .with_timeout(operation.remaining_milliseconds()?),
                            context.prepared.client_data.clone(),
                            operation.cancellation.clone(),
                        )
                        .await
                }
            }
        };
        let result = operation.run_with_cleanup(execute, || stop.cancel()).await;
        stop.cancel();
        result
    }

    pub(super) async fn get_assertions(
        &self,
        mut context: GetAssertionOperationContext<'_>,
    ) -> Result<GetAssertionsResult, FidoClientError> {
        let operation = context
            .operation
            .with_cancellation(context.operation.cancellation.clone());
        let stop = operation.cancellation.child_token();
        context.operation.cancellation = stop.clone();
        let execute = async {
            match &self.backend {
                UsbBackend::Hid(backend) => {
                    let mut target = context.usb_device_id.clone();
                    let mut reconnecting = false;
                    loop {
                        let selected = UsbAuthenticatorSelector::new(backend.as_ref())
                            .select(
                                &context.operation,
                                context.prepared.authenticator_requirements(),
                                crate::AuthenticatorRequest::GetAssertion(
                                    context.prepared.ctap_request(),
                                ),
                                target.as_ref(),
                            )
                            .await;
                        let authenticator = match selected {
                            Ok(authenticator) => authenticator,
                            Err(FidoClientError::NoAuthenticator) if reconnecting => {
                                context
                                    .operation
                                    .run(tokio::time::sleep(std::time::Duration::from_millis(250)))
                                    .await?;
                                continue;
                            }
                            Err(error) => return Err(error),
                        };
                        // Retain the selected device across reconnects, even when the caller
                        // did not supply a target. Never retry against another key.
                        target = Some(
                            authenticator
                                .usb_device_id()
                                .expect("a selected HID authenticator has a device ID")
                                .clone(),
                        );
                        match authenticator.get_assertions(&context).await {
                            Err(error) if is_usb_disconnect(&error) => {
                                tracing::debug!(
                                    ?error,
                                    "waiting for selected USB authenticator to reconnect"
                                );
                                reconnecting = true;
                                context
                                    .operation
                                    .run(tokio::time::sleep(std::time::Duration::from_millis(250)))
                                    .await?;
                            }
                            result => return result,
                        }
                    }
                }
                UsbBackend::Os(backend) => {
                    if context.usb_device_id.is_some() {
                        return Err(FidoClientError::NoAuthenticator);
                    }
                    let operation = &context.operation;
                    backend
                        .get_assertions(
                            operation.id,
                            context
                                .prepared
                                .options
                                .clone()
                                .with_timeout(operation.remaining_milliseconds()?),
                            context.prepared.client_data.clone(),
                            operation.cancellation.clone(),
                        )
                        .await
                }
            }
        };
        let result = operation.run_with_cleanup(execute, || stop.cancel()).await;
        stop.cancel();
        result
    }
}

// Retry only a lost connection. CTAP rejection, protocol errors, and user
// cancellation retain their meaning and must not restart authentication.
fn is_usb_disconnect(error: &FidoClientError) -> bool {
    if !matches!(error, FidoClientError::Transport(_)) {
        return false;
    }
    let mut source = std::error::Error::source(error);
    while let Some(error) = source {
        if let Some(hid) = error.downcast_ref::<crate::usb::UsbHidError>() {
            return hid.is_disconnected();
        }
        source = error.source();
    }
    false
}
