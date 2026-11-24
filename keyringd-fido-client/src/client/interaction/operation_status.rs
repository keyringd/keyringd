use keyringd_fido::AuthenticatorTransport;

use crate::{HybridTransportUnavailableReason, OperationId};

/// Events to notify the UI of the start of an operation, processing with the authenticator, route determination, ending, etc.
/// The selection touch of multiple authenticators and the waiting for approval of registration/authentication after selection will be notified as separate status.
#[derive(Clone)]
pub enum OperationStatus {
    OperationStarted {
        operation_id: OperationId,
    },
    HybridQrReady {
        operation_id: OperationId,
        qr_uri: String,
    },
    AuthenticatorSelectionRequired {
        operation_id: OperationId,
        transport: AuthenticatorTransport,
    },
    UserPresenceRequired {
        operation_id: OperationId,
        transport: AuthenticatorTransport,
    },
    Processing {
        operation_id: OperationId,
        transport: AuthenticatorTransport,
    },
    BuiltInUserVerification {
        operation_id: OperationId,
        transport: AuthenticatorTransport,
    },
    TransportSelected {
        operation_id: OperationId,
        transport: AuthenticatorTransport,
    },
    HybridTransportUnavailable {
        operation_id: OperationId,
        reason: HybridTransportUnavailableReason,
    },
    /// Backend work and cleanup have finished. UI dismissal is owned by the caller.
    BackendFinished {
        operation_id: OperationId,
    },
}

impl OperationStatus {
    pub const fn operation_id(&self) -> OperationId {
        match self {
            Self::OperationStarted { operation_id }
            | Self::HybridQrReady { operation_id, .. }
            | Self::AuthenticatorSelectionRequired { operation_id, .. }
            | Self::UserPresenceRequired { operation_id, .. }
            | Self::Processing { operation_id, .. }
            | Self::BuiltInUserVerification { operation_id, .. }
            | Self::TransportSelected { operation_id, .. }
            | Self::HybridTransportUnavailable { operation_id, .. }
            | Self::BackendFinished { operation_id } => *operation_id,
        }
    }
}
