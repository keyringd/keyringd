use super::KeyOperationEvent;
use crate::runtime::UiEventSender;

use super::KeyOperationKind;

/// Immutable UI routing data captured for one action's interaction dispatch.
pub struct ActionRoute {
    pub events: UiEventSender<KeyOperationEvent>,
    pub operation: KeyOperationKind,
    pub cancellation: tokio_util::sync::CancellationToken,
}
