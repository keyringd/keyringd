use keyringd_ui_core::runtime::{UiEventSender, UiEventTarget};
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};

use windows_reactor::{RenderCx, UiMarshaller};

thread_local! {
    static HANDLERS: RefCell<HashMap<u64, Box<dyn Any>>> = RefCell::new(HashMap::new());
}
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Delivers each event on the UI thread, independently of rendering or value equality.
/// The UI callback stays on that thread; queued events are ignored after unmount.
pub(crate) struct UiEventSink<Event> {
    id: u64,
    marshaller: UiMarshaller,
    marker: PhantomData<fn(Event)>,
}

impl<Event> Clone for UiEventSink<Event> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            marshaller: self.marshaller.clone(),
            marker: PhantomData,
        }
    }
}

impl<Event: Send + 'static> UiEventSink<Event> {
    pub(crate) fn use_in(cx: &mut RenderCx) -> Self {
        let id = cx
            .use_ref(NEXT_ID.fetch_add(1, Ordering::Relaxed))
            .get_cloned();
        cx.use_effect_with_cleanup((), move || {
            Some(move || {
                let removed = HANDLERS.with(|handlers| handlers.borrow_mut().remove(&id));
                drop(removed);
            })
        });
        Self {
            id,
            marshaller: cx.use_ui_marshaller(),
            marker: PhantomData,
        }
    }

    pub(crate) fn bind(&self, handler: UiEventTarget<Event>) {
        let previous =
            HANDLERS.with(|handlers| handlers.borrow_mut().insert(self.id, Box::new(handler)));
        drop(previous);
    }

    pub(crate) fn sender(&self) -> UiEventSender<Event> {
        let id = self.id;
        let marshaller = self.marshaller.clone();
        UiEventSender::new(move |event| {
            let span = tracing::Span::current();
            marshaller.dispatch(move || span.in_scope(|| Self::deliver(id, event)));
        })
    }

    /// A UI input takes effect before the next input is processed.
    pub(crate) fn target(&self) -> UiEventTarget<Event> {
        let id = self.id;
        UiEventTarget::new(move |event| Self::deliver(id, event))
    }

    fn deliver(id: u64, event: Event) {
        // Release the registry borrow before calling user code: a callback may
        // open or close a window and register or remove other event handlers.
        let handler = HANDLERS.with(|handlers| {
            handlers
                .borrow()
                .get(&id)
                .and_then(|handler| handler.downcast_ref::<UiEventTarget<Event>>())
                .cloned()
        });
        if let Some(handler) = handler {
            handler.send(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UiEventSink;
    use keyringd_ui_core::runtime::UiEventTarget;
    use std::cell::RefCell;
    use std::rc::Rc;
    use windows_reactor::{ChannelDispatcher, RenderCx};

    keyringd_testkit::scenario! {
        group "UiEventSink::use_in" {
            case "Delivers equal events before render and ignores events after unmount" {
                verify "deliver repeated events before rendering and ignore post-unmount events" {
                    let dispatcher = ChannelDispatcher::new();
                    let mut cx = RenderCx::new(Rc::new(|| {}));
                    cx.set_marshaller(Some(dispatcher.marshaller()));
                    let events = UiEventSink::use_in(&mut cx);
                    let received = Rc::new(RefCell::new(Vec::new()));
                    let captured = received.clone();
                    events.bind(UiEventTarget::new(move |event| {
                        captured.borrow_mut().push(event)
                    }));
                    cx.flush_effects();

                    events.sender().send(1);
                    events.sender().send(1);
                    events.sender().send(2);
                    assert!(received.borrow().is_empty());
                    dispatcher.drain();
                    assert_eq!(*received.borrow(), vec![1, 1, 2]);

                    events.sender().send(3);
                    cx.run_cleanups();
                    dispatcher.drain();
                    events.target().send(4);
                    assert_eq!(*received.borrow(), vec![1, 1, 2]);
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "UiEventSink::use_in" {
            case "Callback can dispatch another event without borrowing the registry" {
                verify "allow a callback to dispatch again after releasing the registry borrow" {
                    let dispatcher = ChannelDispatcher::new();
                    let mut cx = RenderCx::new(Rc::new(|| {}));
                    cx.set_marshaller(Some(dispatcher.marshaller()));
                    let events = UiEventSink::use_in(&mut cx);
                    let nested = events.target();
                    let received = Rc::new(RefCell::new(Vec::new()));
                    let captured = received.clone();
                    events.bind(UiEventTarget::new(move |event| {
                        captured.borrow_mut().push(event);
                        if event == 1 {
                            nested.send(2);
                        }
                    }));
                    cx.flush_effects();
                    events.target().send(1);
                    assert_eq!(*received.borrow(), vec![1, 2]);
                    cx.run_cleanups();
                }
            }
        }
    }
}
