use keyringd_core::application::ActionId;
use keyringd_ui_core::qr_prompt_window::HybridQrPresentation;
use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::text as ui_text;
use std::sync::Arc;

use windows_reactor::{
    AccessibilityExt, Backdrop, BackgroundExt, Element, GridChildExt, GridLength,
    HorizontalAlignment, InputExt, KeyboardAccelerator, LayoutExt, PaddingExt, ReactorWindow,
    RenderCx, TextStyleExt, ThemeRef, Thickness, Updater, VirtualKey, VirtualKeyModifiers, border,
    button, grid, scroll_viewer, text_block, vstack,
};

use crate::app::ui::util::{
    CreatedReactorWindowLocator, apply_application_icon, center_on_cursor_monitor,
};

use crate::app::AuthenticationPromptCommandHandler;
use crate::app::AuthenticationPromptWindows;
use crate::app::ui::qr_prompt_window::PasskeyInteractionView;
use crate::app::ui::qr_prompt_window::QrPromptCommandHandler;
use keyringd_fido_client::OperationId;
use keyringd_ui_core::key_operation::KeyOperationEvent;

use keyringd_ui_core::qr_prompt_window::QrPromptEvent;
use keyringd_ui_core::qr_prompt_window::QrPromptState;

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;

pub(crate) struct QrPromptWindow;

impl QrPromptWindow {
    pub(crate) fn open(
        operation_id: OperationId,
        flow: ActionId,
        presentation: HybridQrPresentation,
        prompts: AuthenticationPromptCommandHandler,
        session: AuthenticationPromptWindows,
        request_id: keyringd_fido_client::InteractionId,
    ) -> Result<(), crate::app::ui::PromptWindowError> {
        let completed = Arc::new(AtomicBool::new(false));
        let rendered_completed = Arc::clone(&completed);
        let window_handle = Rc::new(Cell::new(0usize));
        let rendered_handle = Rc::clone(&window_handle);
        let locator = CreatedReactorWindowLocator::begin()?;
        ReactorWindow::new()
            .title(presentation.title())
            .inner_size(crate::app::ui::PASSKEY_DIALOG_WIDTH, 660.0)
            .backdrop(Backdrop::Mica)
            .render(move |cx| {
                let (_, set_state) = cx.use_reducer(QrPromptState::default());
                let handler = QrPromptCommandHandler::new(
                    flow,
                    prompts.clone(),
                    session.clone(),
                    request_id,
                    Arc::clone(&rendered_completed),
                    Rc::clone(&rendered_handle),
                    set_state.clone(),
                );
                Self::render(cx, operation_id, &presentation, handler, set_state)
            })?;
        let window = locator.finish()?;
        center_on_cursor_monitor(window);
        apply_application_icon(window)?;
        window_handle.set(window.0 as usize);
        Ok(())
    }

    fn render(
        cx: &mut RenderCx,
        operation_id: OperationId,
        presentation: &HybridQrPresentation,
        handler: QrPromptCommandHandler,
        set_state: Updater<QrPromptState>,
    ) -> Element {
        let (should_close, hybrid_unavailable) = handler.use_session_status(cx);
        cx.use_effect(should_close, {
            let close_set_state = set_state.clone();
            let close_runner = handler.clone();
            move || {
                if should_close {
                    Self::dispatch(&close_set_state, &close_runner, QrPromptEvent::Dismissed);
                }
            }
        });
        let cleanup_runner = handler.clone();
        cx.use_effect_with_cleanup((), move || {
            Some(move || {
                cleanup_runner.on_unmount();
            })
        });
        let usb_set_state = set_state.clone();
        let usb_runner = handler.clone();
        let on_interaction_event: UiEventTarget<KeyOperationEvent> =
            UiEventTarget::new(move |event| match event {
                KeyOperationEvent::UseSecurityKeyRequested => {
                    Self::dispatch(&usb_set_state, &usb_runner, QrPromptEvent::UseUsbRequested)
                }
                event => event.reject(),
            });
        let mut presentation = presentation.clone();
        presentation.unavailable = hybrid_unavailable;
        let interaction =
            PasskeyInteractionView::render(&presentation, operation_id, on_interaction_event);
        let cancel_set_state = set_state.clone();
        let cancel_runner = handler.clone();
        let escape_set_state = set_state;
        let escape_runner = handler;
        grid((
            border(
                vstack((
                    text_block(presentation.title())
                        .automation_id("qr-prompt-window")
                        .font_size(24.0)
                        .semibold()
                        .wrap(),
                    text_block(presentation.explanation())
                        .wrap()
                        .foreground(ThemeRef::SecondaryText),
                ))
                .spacing(8.0),
            )
            .padding(Thickness::xy(24.0, 20.0))
            .grid_row(0),
            scroll_viewer(border(interaction).padding(Thickness::xy(24.0, 12.0))).grid_row(1),
            border(
                button(ui_text::CANCEL)
                    .on_click(move || {
                        Self::dispatch(
                            &cancel_set_state,
                            &cancel_runner,
                            QrPromptEvent::CancelRequested,
                        )
                    })
                    .horizontal_alignment(HorizontalAlignment::Right),
            )
            .padding(Thickness::uniform(16.0))
            .background(ThemeRef::LayerFill)
            .border_brush(ThemeRef::CardStroke)
            .border_thickness(Thickness {
                left: 0.0,
                top: 1.0,
                right: 0.0,
                bottom: 0.0,
            })
            .grid_row(2),
        ))
        .rows([GridLength::Auto, GridLength::STAR, GridLength::Auto])
        .keyboard_accelerator(KeyboardAccelerator::new(
            VirtualKey::Escape,
            VirtualKeyModifiers::None,
            move || {
                Self::dispatch(
                    &escape_set_state,
                    &escape_runner,
                    QrPromptEvent::CancelRequested,
                )
            },
        ))
        .into()
    }

    fn dispatch(
        set_state: &Updater<QrPromptState>,
        handler: &QrPromptCommandHandler,
        event: QrPromptEvent,
    ) {
        let (_, command) = crate::app::ui::reduce(set_state, event);
        if let Some(command) = command {
            handler.handle(command);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::ui::reduce;
    use keyringd_ui_core::qr_prompt_window::QrPromptEvent;
    use keyringd_ui_core::qr_prompt_window::QrPromptState;
    use std::rc::Rc;
    use windows_reactor::RenderCx;

    keyringd_testkit::scenario! {
        group "QrPromptState::update" {
            case "Usb selection then cancel before render does not cancel the operation" {
                verify "ignore cancellation after the USB switch is accepted before rendering" {
                    let mut cx = RenderCx::new(Rc::new(|| {}));
                    let (_, updater) = cx.use_reducer(QrPromptState::default());
                    reduce(&updater, QrPromptEvent::UsbSelected);
                    let (_, command) = reduce(&updater, QrPromptEvent::CancelRequested);
                    assert!(command.is_none());
                }
            }
        }
    }
}
