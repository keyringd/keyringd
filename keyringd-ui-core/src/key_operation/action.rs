use super::{ActionRegistry, KeyOperationEvent};
use crate::runtime::UiEventReceiver;
use keyringd_core::application::ActionId;
use tokio_util::sync::CancellationToken;

/// Owns one action's routing registration and cancellation lifetime.
/// Dropping it cancels the action and unregisters its route, including on task abort.
#[must_use = "the action must remain owned until its work finishes"]
pub struct Action {
    pub(super) id: ActionId,
    pub(super) events: Option<UiEventReceiver<(ActionId, KeyOperationEvent)>>,
    pub(super) cancellation: CancellationToken,
    pub(super) registry: ActionRegistry,
}

impl Action {
    pub fn id(&self) -> ActionId {
        self.id
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }

    pub fn cancel(&self) {
        self.cancellation.cancel();
    }

    /// Takes the initiating UI's event stream once. SSH actions use the shared standalone stream.
    pub fn take_events(&mut self) -> UiEventReceiver<(ActionId, KeyOperationEvent)> {
        self.events
            .take()
            .expect("an owned UI action provides its event stream exactly once")
    }

    /// Keeps registration alive through the complete operation and its cleanup.
    pub async fn run<F: std::future::Future>(self, future: F) -> F::Output {
        let result = self.id.scope(future).await;
        drop(self);
        result
    }
}

impl Drop for Action {
    fn drop(&mut self) {
        self.registry.finish(self.id);
    }
}
