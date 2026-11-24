use super::ctap::CtapOperationExecutor;
use super::operation_context::OperationContext;
use crate::{
    AuthenticatorRequest, AuthenticatorRequirements, FidoClientError, GetAssertionOperationContext,
    GetAssertionsResult, HybridChannelConnection, HybridChannelConnector, HybridOperationHint,
    HybridQrInitiatedTransaction, MakeCredentialOperationContext, MakeCredentialResult,
    OperationStatus,
};
use keyringd_fido::AuthenticatorTransport;
use std::sync::Arc;

/// Hybrid route connection and registration/authentication execution part. The connection procedure is the Connector, and the CTAP operation is delegated to the CTAPOPERATIONEXECUTOR.
/// Responsible for the preparation and presentation of the QR, and the cancellation and cleaning of the connection from the connection to the end of the operation.
pub(super) struct HybridOperationExecutor {
    connector: Arc<dyn HybridChannelConnector>,
}

impl HybridOperationExecutor {
    pub(super) fn new(connector: Arc<dyn HybridChannelConnector>) -> Self {
        Self { connector }
    }

    pub(super) async fn make_credential(
        &self,
        mut context: MakeCredentialOperationContext<'_>,
    ) -> Result<MakeCredentialResult, FidoClientError> {
        let operation = context
            .operation
            .with_cancellation(context.operation.cancellation.clone());
        let transaction = HybridQrInitiatedTransaction::new(HybridOperationHint::MakeCredential)
            .map_err(FidoClientError::protocol)?;
        let stop = operation.cancellation.child_token();
        context.operation.cancellation = stop.clone();
        let execute = async {
            let executor = self
                .connect(
                    &transaction,
                    &context.operation,
                    context.prepared.authenticator_requirements(),
                    AuthenticatorRequest::MakeCredential(context.prepared.ctap_request()),
                )
                .await?;
            executor.make_credential(&context).await
        };
        self.run_operation(
            &operation,
            transaction.qr_uri().to_owned(),
            execute,
            stop.clone(),
        )
        .await
    }

    pub(super) async fn get_assertions(
        &self,
        mut context: GetAssertionOperationContext<'_>,
    ) -> Result<GetAssertionsResult, FidoClientError> {
        let operation = context
            .operation
            .with_cancellation(context.operation.cancellation.clone());
        let transaction = HybridQrInitiatedTransaction::new(HybridOperationHint::GetAssertion)
            .map_err(FidoClientError::protocol)?;
        let stop = operation.cancellation.child_token();
        context.operation.cancellation = stop.clone();
        let execute = async {
            let executor = self
                .connect(
                    &transaction,
                    &context.operation,
                    context.prepared.authenticator_requirements(),
                    AuthenticatorRequest::GetAssertion(context.prepared.ctap_request()),
                )
                .await?;
            executor.get_assertions(&context).await
        };
        self.run_operation(
            &operation,
            transaction.qr_uri().to_owned(),
            execute,
            stop.clone(),
        )
        .await
    }

    async fn run_operation<T, F>(
        &self,
        context: &OperationContext<'_>,
        qr_uri: String,
        operation: F,
        stop: tokio_util::sync::CancellationToken,
    ) -> Result<T, FidoClientError>
    where
        F: std::future::Future<Output = Result<T, FidoClientError>>,
    {
        context
            .handler
            .on_operation_status_changed(OperationStatus::HybridQrReady {
                operation_id: context.id,
                qr_uri,
            })
            .await?;
        let result = context.run_with_cleanup(operation, || stop.cancel()).await;
        stop.cancel();
        result
    }

    async fn connect(
        &self,
        transaction: &HybridQrInitiatedTransaction,
        operation: &OperationContext<'_>,
        requirements: &AuthenticatorRequirements,
        request: AuthenticatorRequest<'_>,
    ) -> Result<CtapOperationExecutor, FidoClientError> {
        let connection = self
            .connector
            .connect(transaction, operation.cancellation.clone())
            .await?;
        let channel = match connection {
            HybridChannelConnection::Connected(channel) => channel,
            HybridChannelConnection::Unavailable(reason) => {
                operation
                    .handler
                    .on_operation_status_changed(OperationStatus::HybridTransportUnavailable {
                        operation_id: operation.id,
                        reason,
                    })
                    .await?;
                return Err(FidoClientError::NoAuthenticator);
            }
        };
        CtapOperationExecutor::connect(
            channel,
            AuthenticatorTransport::hybrid(),
            requirements,
            request,
            operation.cancellation.clone(),
            None,
        )
        .await
    }
}
