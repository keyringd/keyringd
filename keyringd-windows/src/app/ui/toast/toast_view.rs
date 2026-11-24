use windows_reactor::{
    AccessibilityExt, AutomationLiveSetting, DispatcherTimer, Element, HorizontalAlignment,
    InfoBar, InfoBarSeverity, KeyExt, LayoutExt, RenderCx, Thickness, VerticalAlignment,
};

use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::toast::{ToastMessage, ToastState};

pub(in crate::app::ui) fn use_toast(cx: &mut RenderCx) -> (Element, UiEventTarget<ToastMessage>) {
    let (state, update_state) = cx.use_reducer(ToastState::default());
    let expire_state = update_state.clone();
    let revision = state.revision();
    let timeout = state.auto_dismiss_after();
    cx.use_effect_with_cleanup(state.clone(), move || {
        let timer = timeout.and_then(|timeout| {
            DispatcherTimer::new_one_shot(timeout, move || {
                expire_state.call(move |mut state| {
                    state.expire(revision);
                    state
                });
            })
            .map_err(|error| tracing::warn!(%error, "could not schedule toast dismissal"))
            .ok()
        });
        Some(move || drop(timer))
    });
    let dismiss_state = update_state.clone();
    let feedback: Element = match state.message() {
        Some(message) => InfoBar::new(message.text())
            .automation_id("feedback-toast-message")
            .automation_name(message.text())
            .severity(if message.is_error() {
                InfoBarSeverity::Error
            } else {
                InfoBarSeverity::Success
            })
            .is_open(true)
            .is_closable(true)
            .on_closed(move || {
                dismiss_state.call(move |mut state| {
                    state.dismiss(revision);
                    state
                });
            })
            .horizontal_alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Bottom)
            .margin(Thickness::uniform(16.0))
            .accessibility_live_setting(AutomationLiveSetting::Polite)
            .into(),
        None => Element::Empty,
    };
    let on_toast = UiEventTarget::new(move |message| {
        update_state.call(move |mut state| {
            state.show(message);
            state
        });
    });
    (feedback.with_key(format!("toast-{revision}")), on_toast)
}
