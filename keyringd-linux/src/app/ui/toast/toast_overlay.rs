use gtk4::glib::object::ObjectExt;
use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::toast::{ToastMessage, ToastState};
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) struct ToastOverlay {
    widget: libadwaita::ToastOverlay,
    state: Rc<RefCell<NativeToastState>>,
}

#[derive(Default)]
struct NativeToastState {
    model: ToastState,
    current: Option<libadwaita::Toast>,
}

impl ToastOverlay {
    pub(crate) fn new() -> Self {
        Self {
            widget: libadwaita::ToastOverlay::new(),
            state: Rc::new(RefCell::new(NativeToastState::default())),
        }
    }

    pub(crate) fn widget(&self) -> &libadwaita::ToastOverlay {
        &self.widget
    }

    pub(crate) fn event_target(&self) -> UiEventTarget<ToastMessage> {
        let overlay = self.widget.downgrade();
        let state = Rc::clone(&self.state);
        UiEventTarget::new(move |message: ToastMessage| {
            let Some(overlay) = overlay.upgrade() else {
                return;
            };
            let (revision, timeout, previous) = {
                let mut state = state.borrow_mut();
                let revision = state.model.show(message.clone());
                (
                    revision,
                    state.model.auto_dismiss_after(),
                    state.current.take(),
                )
            };
            if let Some(previous) = previous {
                previous.dismiss();
            }
            let toast = libadwaita::Toast::builder()
                .title(message.text())
                .priority(if message.is_error() {
                    libadwaita::ToastPriority::High
                } else {
                    libadwaita::ToastPriority::Normal
                })
                .timeout(timeout.map_or(0, |duration| duration.as_secs() as u32))
                .build();
            let weak_state = Rc::downgrade(&state);
            toast.connect_dismissed(move |_| {
                if let Some(state) = weak_state.upgrade() {
                    let mut state = state.borrow_mut();
                    if state.model.dismiss(revision) {
                        state.current.take();
                    }
                }
            });
            state.borrow_mut().current = Some(toast.clone());
            overlay.add_toast(toast);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "ToastOverlay::new" {
            case "Repeated toasts survive old native dismissals and owner scope exit" {
                verify "keep the latest toast visible through stale native completions and owner exit" {
                    gtk4::test_synced(|| {
                        libadwaita::init().unwrap();
                        let overlay = ToastOverlay::new();
                        let widget = overlay.widget().clone();
                        let state = Rc::clone(&overlay.state);
                        let show = overlay.event_target();
                        drop(overlay);

                        show.send(ToastMessage::Success("Copied".into()));
                        let first = state.borrow().current.clone().unwrap();
                        assert_eq!(first.timeout(), 3);
                        show.send(ToastMessage::Success("Copied".into()));
                        let revision = state.borrow().model.revision();
                        let second = state.borrow().current.clone().unwrap();
                        assert_ne!(first, second);
                        first.emit_by_name::<()>("dismissed", &[]);
                        assert_eq!(state.borrow().model.revision(), revision);
                        assert!(state.borrow().model.message().is_some());

                        show.send(ToastMessage::Error("Copy failed".into()));
                        let error = state.borrow().current.clone().unwrap();
                        assert_eq!(error.timeout(), 0);
                        let revision = state.borrow().model.revision();
                        assert!(!state.borrow_mut().model.expire(revision));
                        error.emit_by_name::<()>("dismissed", &[]);
                        assert!(state.borrow().model.message().is_none());
                        assert!(state.borrow().current.is_none());
                        drop(widget);
                    });
                }
            }
        }
    }
}
