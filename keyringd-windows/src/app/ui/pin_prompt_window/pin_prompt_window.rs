use super::PinPromptView;
use crate::app::AuthenticationPromptWindows;
use crate::app::ui::util::{
    CreatedReactorWindowLocator, apply_application_icon, center_on_cursor_monitor,
};
use keyringd_ui_core::pin_prompt_window::{PinPromptCommand, SecurityKeyPinPrompt};
use keyringd_ui_core::runtime::{UiEventReceiver, UiEventTarget};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{PostMessageW, WM_CLOSE};
use windows_reactor::{Backdrop, Element, ReactorWindow, RenderCx};

pub(crate) struct PinPromptWindow {
    response: Rc<RefCell<Option<tokio::sync::oneshot::Sender<Option<String>>>>>,
    window_handle: Rc<Cell<usize>>,
    runtime: tokio::runtime::Handle,
    session: AuthenticationPromptWindows,
    request_id: keyringd_fido_client::InteractionId,
    events: keyringd_ui_core::runtime::UiEventSender<
        keyringd_ui_core::key_operation::KeyOperationEvent,
    >,
}

impl PinPromptWindow {
    pub(crate) fn open(
        request: SecurityKeyPinPrompt,
        response: tokio::sync::oneshot::Sender<Option<String>>,
        runtime: tokio::runtime::Handle,
        session: AuthenticationPromptWindows,
        request_id: keyringd_fido_client::InteractionId,
        events: keyringd_ui_core::runtime::UiEventSender<
            keyringd_ui_core::key_operation::KeyOperationEvent,
        >,
    ) -> Result<(), crate::app::ui::PromptWindowError> {
        let initial_height = if request.new_pin { 390.0 } else { 300.0 };
        let locator = CreatedReactorWindowLocator::begin()?;
        let window_handle = Rc::new(Cell::new(0usize));
        let window = Self {
            response: Rc::new(RefCell::new(Some(response))),
            window_handle: Rc::clone(&window_handle),
            runtime,
            session,
            request_id,
            events,
        };
        ReactorWindow::new()
            .title(keyringd_ui_core::pin_prompt_window::pin_heading(
                request.new_pin,
            ))
            .inner_size(420.0, initial_height)
            .backdrop(Backdrop::Mica)
            .render(move |cx| window.render(cx, &request))?;
        let window = locator.finish()?;
        center_on_cursor_monitor(window);
        apply_application_icon(window)?;
        window_handle.set(window.0 as usize);
        Ok(())
    }

    fn render(&self, cx: &mut RenderCx, prompt: &SecurityKeyPinPrompt) -> Element {
        let Self {
            response,
            window_handle,
            runtime,
            session,
            request_id,
            events,
        } = self;
        let (response, window_handle, runtime, session, request_id, events) = (
            Rc::clone(response),
            Rc::clone(window_handle),
            runtime.clone(),
            session.clone(),
            *request_id,
            events.clone(),
        );
        let mut changes = session.subscribe();
        let initial = *changes.borrow_and_update();
        let (session_state, set_session_state) = cx.use_async_state(initial);
        cx.use_effect_with_cleanup((), move || {
            let (initial, mut changes) = UiEventReceiver::from_tokio_watch(&runtime, changes);
            set_session_state.call(initial);
            let task = runtime.spawn(async move {
                while let Some(state) = changes.receive().await {
                    set_session_state.call(state);
                }
            });
            Some(move || task.abort())
        });
        let cleanup_response = response.clone();
        let cleanup_events = events.clone();
        cx.use_effect_with_cleanup((), move || {
            Some(move || Self::respond(&cleanup_response, request_id, &cleanup_events, None))
        });
        let completion = UiEventTarget::new(move |command| {
            Self::complete(&response, &window_handle, request_id, &events, command)
        });
        PinPromptView::render(
            prompt,
            request_id,
            !session_state.is_active(request_id),
            completion,
        )
    }

    fn complete(
        response: &RefCell<Option<tokio::sync::oneshot::Sender<Option<String>>>>,
        window_handle: &Cell<usize>,
        request_id: keyringd_fido_client::InteractionId,
        events: &keyringd_ui_core::runtime::UiEventSender<
            keyringd_ui_core::key_operation::KeyOperationEvent,
        >,
        command: PinPromptCommand,
    ) {
        let value = match command {
            PinPromptCommand::Submit(pin) => Some(pin),
            PinPromptCommand::Cancel | PinPromptCommand::Dismiss => None,
        };
        Self::respond(response, request_id, events, value);
        let handle = window_handle.get();
        if handle != 0 {
            unsafe {
                let _ = PostMessageW(Some(HWND(handle as *mut _)), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
    }
    fn respond(
        response: &RefCell<Option<tokio::sync::oneshot::Sender<Option<String>>>>,
        request_id: keyringd_fido_client::InteractionId,
        events: &keyringd_ui_core::runtime::UiEventSender<
            keyringd_ui_core::key_operation::KeyOperationEvent,
        >,
        value: Option<String>,
    ) {
        if let Some(response) = response.borrow_mut().take() {
            events.send(
                keyringd_ui_core::key_operation::KeyOperationEvent::PinAnswered {
                    request_id,
                    value,
                    response,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ui::pin_prompt_window::PinPromptView;
    use keyringd_fido_client::{InteractionId, OperationId};
    use keyringd_ui_core::pin_prompt_window::PinPromptEvent;
    use keyringd_ui_core::pin_prompt_window::PinPromptState;
    use keyringd_ui_core::{key_operation::KeyOperationEvent, runtime::UiEventSender};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    use windows_reactor::RenderCx;

    fn complete_answer(event: KeyOperationEvent, expected: InteractionId) {
        let KeyOperationEvent::PinAnswered {
            request_id,
            value,
            response,
        } = event
        else {
            panic!("expected a PIN answer event");
        };
        assert_eq!(request_id, expected);
        response.send(value).unwrap();
    }

    keyringd_testkit::scenario! {
        group "PinPromptWindow::complete" {
            case "Input and submit before render deliver the latest pin once" {
                verify "send the last entered PIN once even when submit precedes rendering" {
                    let mut cx = RenderCx::new(Rc::new(|| {}));
                    let (_, updater) = cx.use_reducer(PinPromptState::new(4));
                    let (sender, mut receiver) = tokio::sync::oneshot::channel();
                    let request_id = InteractionId::new(OperationId::new(1), 1);
                    let response = Rc::new(RefCell::new(Some(sender)));
                    let (events, mut key_events) = tokio::sync::mpsc::unbounded_channel();
                    let events = UiEventSender::from_tokio(events);
                    let completion = UiEventTarget::new(move |command| {
                        PinPromptWindow::complete(&response, &Cell::new(0), request_id, &events, command)
                    });
                    PinPromptView::dispatch(
                        &updater,
                        &completion,
                        PinPromptEvent::PinChanged("1234".into()),
                    );
                    PinPromptView::dispatch(&updater, &completion, PinPromptEvent::SubmitRequested);
                    assert!(matches!(
                        receiver.try_recv(),
                        Err(tokio::sync::oneshot::error::TryRecvError::Empty)
                    ));
                    complete_answer(key_events.try_recv().unwrap(), request_id);
                    assert_eq!(receiver.try_recv().unwrap().as_deref(), Some("1234"));
                    PinPromptView::dispatch(&updater, &completion, PinPromptEvent::CancelRequested);
                    cx.begin_render();
                    let (state, _) = cx.use_reducer(PinPromptState::new(4));
                    assert!(!state.can_submit());
                    assert!(key_events.try_recv().is_err());
                }
            }
        }
    }
    keyringd_testkit::scenario! {
        group "PinPromptWindow::complete" {
            case "New pin confirmation must match before native response is sent" {
                verify "block native submission until the new PIN and confirmation agree" {
                    let prompt = keyringd_ui_core::pin_prompt_window::SecurityKeyPinPrompt {
                        reason: String::new(),
                        attempts_left: None,
                        minimum_length: Some(4),
                        new_pin: true,
                    };
                    let mut cx = RenderCx::new(Rc::new(|| {}));
                    let (_, updater) = cx.use_reducer(PinPromptState::for_prompt(&prompt));
                    let (sender, mut receiver) = tokio::sync::oneshot::channel();
                    let request_id = InteractionId::new(OperationId::new(1), 1);
                    let response = Rc::new(RefCell::new(Some(sender)));
                    let (events, mut key_events) = tokio::sync::mpsc::unbounded_channel();
                    let events = UiEventSender::from_tokio(events);
                    let completion = UiEventTarget::new(move |command| {
                        PinPromptWindow::complete(&response, &Cell::new(0), request_id, &events, command)
                    });
                    PinPromptView::dispatch(
                        &updater,
                        &completion,
                        PinPromptEvent::PinChanged("1234".into()),
                    );
                    PinPromptView::dispatch(
                        &updater,
                        &completion,
                        PinPromptEvent::ConfirmationChanged("5678".into()),
                    );
                    PinPromptView::dispatch(&updater, &completion, PinPromptEvent::SubmitRequested);
                    assert!(matches!(
                        receiver.try_recv(),
                        Err(tokio::sync::oneshot::error::TryRecvError::Empty)
                    ));
                    assert!(key_events.try_recv().is_err());
                    PinPromptView::dispatch(
                        &updater,
                        &completion,
                        PinPromptEvent::ConfirmationChanged("1234".into()),
                    );
                    PinPromptView::dispatch(&updater, &completion, PinPromptEvent::SubmitRequested);
                    complete_answer(key_events.try_recv().unwrap(), request_id);
                    assert_eq!(receiver.try_recv().unwrap().as_deref(), Some("1234"));
                    PinPromptView::dispatch(&updater, &completion, PinPromptEvent::SubmitRequested);
                    assert!(key_events.try_recv().is_err());
                }
            }
        }
    }
}
