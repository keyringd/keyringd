/// Receives background events on either a Tokio or native UI executor.
pub struct UiEventReceiver<T> {
    receiver: tokio::sync::mpsc::UnboundedReceiver<T>,
}

impl<T> UiEventReceiver<T> {
    pub fn from_tokio(receiver: tokio::sync::mpsc::UnboundedReceiver<T>) -> Self {
        Self { receiver }
    }

    pub async fn receive(&mut self) -> Option<T> {
        // Native UI executors do not replenish Tokio cooperative task budgets.
        tokio::task::unconstrained(self.receiver.recv()).await
    }
}

impl<T: Clone + Send + Sync + 'static> UiEventReceiver<T> {
    /// Read the initial value before spawning, from the same subscription that
    /// delivers subsequent changes. A separate getter can miss an intervening update.
    pub fn from_tokio_watch(
        runtime: &tokio::runtime::Handle,
        mut receiver: tokio::sync::watch::Receiver<T>,
    ) -> (T, Self) {
        let initial = receiver.borrow_and_update().clone();
        let (sender, events) = tokio::sync::mpsc::unbounded_channel();
        runtime.spawn(async move {
            loop {
                tokio::select! {
                    _ = sender.closed() => break,
                    change = receiver.changed() => {
                        if change.is_err() { break; }
                    }
                }
                if sender.send(receiver.borrow_and_update().clone()).is_err() {
                    break;
                }
            }
        });
        (initial, Self::from_tokio(events))
    }
}

#[cfg(test)]
mod tests {
    use super::UiEventReceiver;

    keyringd_testkit::scenario! {
        group "UiEventReceiver::receive" {
            case "Receives on a native executor without a tokio runtime" {
                verify "await an event on the native executor without entering a Tokio runtime" {
                    use std::future::Future;
                    use std::task::{Context, Poll, Waker};

                    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
                    let mut events = UiEventReceiver::from_tokio(receiver);
                    for value in 0..1_024 {
                        sender.send(value).unwrap();
                    }
                    drop(sender);
                    let mut context = Context::from_waker(Waker::noop());
                    for expected in 0..1_024 {
                        let mut next = std::pin::pin!(events.receive());
                        assert_eq!(
                            next.as_mut().poll(&mut context),
                            Poll::Ready(Some(expected))
                        );
                    }
                    let mut next = std::pin::pin!(events.receive());
                    assert_eq!(next.as_mut().poll(&mut context), Poll::Ready(None));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UiEventReceiver::from_tokio" {
            case "Transfer Events that exceeds Tokio's Cooperative Budget" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                given "Prepare a UIEventReceiver" {
                    const EVENT_COUNT: usize = 1_024;
                    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
                    let mut ui_events = UiEventReceiver::from_tokio(receiver);
                }
                when "Call sender.send" {
                    for event in 0..EVENT_COUNT {
                        sender.send(event).unwrap();
                    }
                    drop(sender);
                }
                then "Transfer Events that exceeds Tokio's Cooperative Budget" {
                    for expected in 0..EVENT_COUNT {
                        assert_eq!(ui_events.receive().await, Some(expected));
                    }
                    assert_eq!(ui_events.receive().await, None);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UiEventReceiver::from_tokio_watch" {
            case "Forwards watch updates as owned UI events" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                given "Prepare a Watch Channel and UI Receiver with initial values" {
                    let (sender, receiver) = tokio::sync::watch::channel(String::from("initial"));
                    let (initial, mut ui_events) =
                        UiEventReceiver::from_tokio_watch(&tokio::runtime::Handle::current(), receiver);
                }
                when "Update watch values" {
                    sender.send(String::from("changed")).unwrap();
                    let event = ui_events.receive().await;
                }
                then "Receive updated values as Ownership UI Events" {
                    assert_eq!(initial, "initial");
                    assert_eq!(event.as_deref(), Some("changed"));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UiEventReceiver::from_tokio_watch" {
            case "Receives the latest value on subscription and all subsequent updates" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                given "Watch is updated after reading previous Snapshot" {
                    let (sender, _) = tokio::sync::watch::channel(String::from("old"));
                    let old_snapshot = sender.borrow().clone();
                    sender.send_replace(String::from("before subscription"));
                }
                when "Start subscribing to UI and update Task again before launching Task" {
                    let (initial, mut ui_events) =
                        UiEventReceiver::from_tokio_watch(&tokio::runtime::Handle::current(), sender.subscribe());
                    sender.send_replace(String::from("after subscription"));
                    drop(sender);
                }
                then "Get initial values and last updates for the same subscription in order" {
                    assert_eq!(old_snapshot, "old");
                    assert_eq!(initial, "before subscription");
                    assert_eq!(
                        ui_events.receive().await.as_deref(),
                        Some("after subscription")
                    );
                    assert_eq!(ui_events.receive().await, None);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UiEventReceiver::from_tokio_watch" {
            case "Delivers an unread initial update only once" {
                async: tokio;
                timeout: std::time::Duration::from_secs(10);
                given "Subscribed Receiver has unread updates" {
                    let (sender, receiver) = tokio::sync::watch::channel(String::from("old"));
                    sender.send(String::from("latest")).unwrap();
                }
                when "Get the initial value and close the update source" {
                    let (initial, mut ui_events) =
                        UiEventReceiver::from_tokio_watch(&tokio::runtime::Handle::current(), receiver);
                    drop(sender);
                }
                then "Returns the latest initial value and exits without sending duplicate Events" {
                    assert_eq!(initial, "latest");
                    assert_eq!(ui_events.receive().await, None);
                }
            }
        }
    }
}
