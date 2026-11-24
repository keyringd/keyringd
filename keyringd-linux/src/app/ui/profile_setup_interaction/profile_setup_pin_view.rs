use crate::app::ui::page_header::PageHeaderView;
use crate::app::ui::pin_prompt_window::PinPromptView;
use crate::app::ui::profile_setup_interaction::interaction_body;
use gtk4::glib::object::{Cast, ObjectExt};
use gtk4::prelude::{AccessibleExtManual, BoxExt, ButtonExt, WidgetExt};
use keyringd_ui_core::pin_prompt_window::{PinPromptEvent, SecurityKeyPinPrompt};
use keyringd_ui_core::profile_setup_window::ProfileSetupEvent;
use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::text as ui_text;
use std::cell::Cell;
use std::rc::Rc;

pub(super) struct ProfileSetupPinView {
    widget: gtk4::Widget,
    pub(super) cancel: gtk4::Button,
    pub(super) header: PageHeaderView,
    view: Rc<PinPromptView>,
    request_id: Rc<Cell<Option<keyringd_fido_client::InteractionId>>>,
}

impl ProfileSetupPinView {
    pub(super) fn new(events: UiEventTarget<ProfileSetupEvent>) -> Self {
        let body = interaction_body();
        let cancel = gtk4::Button::with_label(ui_text::CANCEL);
        let continue_pin = gtk4::Button::with_label(ui_text::CONTINUE);
        continue_pin.add_css_class("suggested-action");
        let weak_continue = continue_pin.downgrade();
        let request_id = Rc::new(Cell::new(None));
        let response_request = request_id.clone();
        let view = PinPromptView::new(
            UiEventTarget::new(move |command| {
                if let Some(request_id) = response_request.get() {
                    events.send(ProfileSetupEvent::PinCompleted {
                        request_id,
                        command,
                    });
                }
            }),
            UiEventTarget::new(move |enabled| {
                if let Some(button) = weak_continue.upgrade() {
                    button.set_sensitive(enabled);
                }
            }),
        );
        continue_pin.set_sensitive(false);
        let weak_view = Rc::downgrade(&view);
        continue_pin.connect_clicked(move |_| {
            if let Some(view) = weak_view.upgrade() {
                view.dispatch(PinPromptEvent::SubmitRequested);
            }
        });
        let weak_view = Rc::downgrade(&view);
        cancel.connect_clicked(move |_| {
            if let Some(view) = weak_view.upgrade() {
                view.dispatch(PinPromptEvent::CancelRequested);
            }
        });
        let actions = gtk4::ActionBar::new();
        actions.pack_end(&continue_pin);
        actions.pack_end(&cancel);
        body.append(&view.root);
        let header = PageHeaderView::new();
        let wrapper = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        wrapper.append(&header.root);
        wrapper.append(&body);
        let page = libadwaita::ToolbarView::builder()
            .accessible_role(gtk4::AccessibleRole::Group)
            .build();
        page.set_content(Some(&wrapper));
        page.add_bottom_bar(&actions);
        Self {
            widget: page.upcast(),
            cancel,
            header,
            view,
            request_id,
        }
    }
    pub(super) fn widget(&self) -> &gtk4::Widget {
        &self.widget
    }
    pub(super) fn cancel_pending(&self) {
        self.view.dispatch(PinPromptEvent::Dismissed);
    }
    pub(super) fn clear_request(&self) {
        self.request_id.set(None);
    }
    pub(super) fn focus(&self) {
        self.view.focus();
    }
    pub(super) fn render(
        &self,
        prompt: &SecurityKeyPinPrompt,
        request_id: Option<keyringd_fido_client::InteractionId>,
    ) {
        self.widget
            .update_property(&[gtk4::accessible::Property::Label(
                keyringd_ui_core::pin_prompt_window::pin_heading(prompt.new_pin),
            )]);
        self.view.render(prompt);
        if self.request_id.replace(request_id) != request_id {
            self.view.restart();
        }
    }
}
