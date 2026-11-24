use super::{UiEventReceiver, UiEventSendError};
use std::sync::Arc;

/// Thread-safe notifications; native delivery adapters own UI scheduling.
pub struct UiEventSender<T> {
    deliver: Arc<dyn Fn(T) -> Result<(), UiEventSendError> + Send + Sync>,
}

impl<T> Clone for UiEventSender<T> {
    fn clone(&self) -> Self {
        Self {
            deliver: Arc::clone(&self.deliver),
        }
    }
}

impl<T: Send + 'static> UiEventSender<T> {
    pub fn new(deliver: impl Fn(T) + Send + Sync + 'static) -> Self {
        Self {
            deliver: Arc::new(move |event| {
                deliver(event);
                Ok(())
            }),
        }
    }
    pub fn from_tokio(sender: tokio::sync::mpsc::UnboundedSender<T>) -> Self {
        Self {
            deliver: Arc::new(move |event| sender.send(event).map_err(|_| UiEventSendError)),
        }
    }
    pub fn channel() -> (Self, UiEventReceiver<T>) {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        (
            Self::from_tokio(sender),
            UiEventReceiver::from_tokio(receiver),
        )
    }
    /// Delivery adapters discard notifications after the component is disposed.
    pub fn send(&self, event: T) {
        let _ = self.try_send(event);
    }
    /// Use for requests whose caller must detect a disconnected UI.
    pub fn try_send(&self, event: T) -> Result<(), UiEventSendError> {
        (self.deliver)(event)
    }
    pub fn map<U: Send + 'static>(
        &self,
        map: impl Fn(U) -> T + Send + Sync + 'static,
    ) -> UiEventSender<U> {
        let sender = self.clone();
        UiEventSender {
            deliver: Arc::new(move |event| sender.try_send(map(event))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UiEventSender;
    use std::future::Future;
    use std::task::{Context, Poll, Waker};
    keyringd_testkit::scenario! {
        group "UiEventSender::map" {
            case "Mapped events preserve duplicates and order across threads" {
                verify "preserve repeated events and send order across the thread boundary" {
                    let (events, mut receiver) = UiEventSender::channel();
                    let selected = events.map(|index: usize| format!("selected:{index}"));
                    std::thread::spawn(move || {
                        selected.send(1);
                        selected.send(1);
                        selected.send(2);
                    })
                    .join()
                    .unwrap();
                    drop(events);
                    let mut context = Context::from_waker(Waker::noop());
                    for expected in ["selected:1", "selected:1", "selected:2"] {
                        let mut next = std::pin::pin!(receiver.receive());
                        assert_eq!(
                            next.as_mut().poll(&mut context),
                            Poll::Ready(Some(expected.to_owned()))
                        );
                    }
                    let mut next = std::pin::pin!(receiver.receive());
                    assert_eq!(next.as_mut().poll(&mut context), Poll::Ready(None));
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "UiEventSender::channel" {
            case "Notifications after receiver disposal are harmless" {
                verify "safely discard notifications after the receiver has been dropped" {
                    let (events, receiver) = UiEventSender::<usize>::channel();
                    drop(receiver);
                    assert!(events.try_send(1).is_err());
                    assert!(events.map(|value: usize| value + 1).try_send(1).is_err());
                    events.send(1);
                }
            }
        }
    }
}
