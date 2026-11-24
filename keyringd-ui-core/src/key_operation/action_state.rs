use super::KeyOperationEvent;
use crate::runtime::UiEventSender;

use super::KeyOperationKind;

/// Mutable routing state belonging to exactly one key-operation flow.
pub(super) struct ActionState {
    pub(super) span: tracing::Span,
    pub(super) operation: KeyOperationKind,
    pub(super) events: UiEventSender<KeyOperationEvent>,
    pub(super) cancellation: tokio_util::sync::CancellationToken,
    pub(super) _standalone_guard: Option<tokio::sync::OwnedMutexGuard<()>>,
}
