use crate::NewPinInteraction;
use crate::{FidoClientError, FidoInteractionHandler, OperationId};
use crate::{OperationStatus, PinInteraction};
use keyringd_fido::AuthenticatorTransport;
use keyringd_fido::ctap::Pin;
use std::time::Instant;
use tokio_util::sync::CancellationToken;

/// Pass the ID, deadline, cancellation status, and interactive handler required to perform a single operation to the Authenticator candidate.
pub(crate) struct OperationContext<'a> {
    pub(crate) id: OperationId,
    pub(crate) deadline: Instant,
    pub(crate) cancellation: CancellationToken,
    pub(crate) handler: &'a dyn FidoInteractionHandler,
}

impl<'a> OperationContext<'a> {
    pub(crate) fn with_cancellation(
        &self,
        cancellation: CancellationToken,
    ) -> OperationContext<'a> {
        OperationContext {
            id: self.id,
            deadline: self.deadline,
            cancellation,
            handler: self.handler,
        }
    }
    pub(in crate::client::operation) async fn run<F, T>(
        &self,
        operation: F,
    ) -> Result<T, FidoClientError>
    where
        F: std::future::Future<Output = T>,
    {
        tokio::select! {
            _ = self.cancellation.cancelled() => Err(FidoClientError::Cancelled),
            result = tokio::time::timeout_at(tokio::time::Instant::from_std(self.deadline), operation) => {
                result.map_err(|_| FidoClientError::TimedOut)
            }
        }
    }

    // When keepalive is waiting for up, the command execution source specifies one of the selection, approval, and internal UVs.
    pub(in crate::client::operation) async fn run_with_status<F, T, E>(
        &self,
        transport: &AuthenticatorTransport,
        confirmation: OperationStatus,
        mut statuses: Option<&mut tokio::sync::mpsc::UnboundedReceiver<crate::CtapTransportStatus>>,
        operation: F,
    ) -> Result<Result<T, E>, FidoClientError>
    where
        F: std::future::Future<Output = Result<T, E>>,
    {
        let handler = self.handler;
        let operation_id = self.id;
        tokio::pin!(operation);
        let timeout = tokio::time::sleep_until(tokio::time::Instant::from_std(self.deadline));
        tokio::pin!(timeout);
        loop {
            tokio::select! {
                _ = self.cancellation.cancelled() => {
                    return Err(FidoClientError::Cancelled);
                }
                _ = &mut timeout => return Err(FidoClientError::TimedOut),
                Some(status) = async {
                    match statuses.as_deref_mut() {
                        Some(receiver) => receiver.recv().await,
                        None => std::future::pending().await,
                    }
                } => {
                    let event = match status {
                        crate::CtapTransportStatus::Processing => {
                            OperationStatus::Processing {
                                operation_id,
                                transport: transport.clone(),
                            }
                        }
                        crate::CtapTransportStatus::UserPresenceNeeded => {
                            confirmation.clone()
                        }
                    };
                    handler.on_operation_status_changed(event).await?;
                }
                result = &mut operation => return Ok(result),
            }
        }
    }

    pub(in crate::client::operation) fn user_confirmation_status(
        &self,
        transport: &AuthenticatorTransport,
        built_in_uv: bool,
    ) -> OperationStatus {
        let operation_id = self.id;
        if built_in_uv {
            OperationStatus::BuiltInUserVerification {
                operation_id,
                transport: transport.clone(),
            }
        } else {
            OperationStatus::UserPresenceRequired {
                operation_id,
                transport: transport.clone(),
            }
        }
    }

    pub(in crate::client::operation) async fn request_pin(
        &self,
        interaction: PinInteraction,
    ) -> Result<Option<Pin>, FidoClientError> {
        interaction
            .id()
            .scope(async {
                tracing::debug!("requesting PIN interaction");
                let response = self.run(self.handler.request_pin(interaction)).await??;
                tracing::debug!("PIN interaction response received");
                if response.id().operation_id() != self.id {
                    return Ok(None);
                }
                response
                    .into_value()
                    .map(Some)
                    .ok_or(FidoClientError::Cancelled)
            })
            .await
    }

    pub(in crate::client::operation) async fn request_new_pin(
        &self,
        interaction: NewPinInteraction,
    ) -> Result<Option<Pin>, FidoClientError> {
        interaction
            .id()
            .scope(async {
                let response = self
                    .run(self.handler.request_new_pin(interaction))
                    .await??;
                if response.id().operation_id() != self.id {
                    return Ok(None);
                }
                response
                    .into_value()
                    .map(Some)
                    .ok_or(FidoClientError::Cancelled)
            })
            .await
    }

    pub(in crate::client::operation) fn remaining_milliseconds(
        &self,
    ) -> Result<u32, FidoClientError> {
        let remaining = self
            .deadline
            .saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err(FidoClientError::TimedOut);
        }
        Ok(remaining.as_millis().clamp(1, u128::from(u32::MAX)) as u32)
    }

    pub(in crate::client::operation) async fn run_with_cleanup<T, F>(
        &self,
        operation: F,
        cancel: impl FnOnce(),
    ) -> Result<T, FidoClientError>
    where
        F: std::future::Future<Output = Result<T, FidoClientError>>,
    {
        tokio::pin!(operation);
        match self.run(&mut operation).await {
            Ok(result) => result,
            Err(error) => {
                cancel();
                // Let an in-flight channel send its cancellation before dropping the future.
                let _ = tokio::time::timeout(std::time::Duration::from_millis(250), &mut operation)
                    .await;
                Err(error)
            }
        }
    }
}
