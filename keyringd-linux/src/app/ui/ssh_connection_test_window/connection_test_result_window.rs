use crate::app::ui::toast::ToastOverlay;
use gtk4::prelude::{AccessibleExtManual, BoxExt, ButtonExt, GtkWindowExt, WidgetExt};
use keyringd_core::service::SshConnectionTestReport;
use keyringd_ui_core::ssh_connection_test_window::{
    SshConnectionTestEvent, SshConnectionTestPresentation,
};
use keyringd_ui_core::text as ui_text;
use keyringd_ui_core::toast::ToastMessage;
use libadwaita::prelude::MessageDialogExt;

pub(in crate::app::ui) struct ConnectionTestResultWindow;

impl ConnectionTestResultWindow {
    pub(in crate::app::ui) fn present(
        parent: &gtk4::Window,
        target: &str,
        report: &SshConnectionTestReport,
        on_event: std::rc::Rc<dyn Fn(SshConnectionTestEvent) -> Option<ToastMessage>>,
    ) {
        let presentation = SshConnectionTestPresentation::new(report);
        let body = presentation.body_with_host(target);
        let SshConnectionTestPresentation {
            heading,
            troubleshooting,
            ..
        } = presentation;
        let authorized_keys_heading = gtk4::Label::builder()
            .label(ui_text::SSH_CONNECTION_TEST_AUTHORIZED_KEYS_HEADING)
            .xalign(0.0)
            .build();
        authorized_keys_heading.add_css_class("heading");
        let authorized_keys_explanation = gtk4::Label::builder()
            .label(ui_text::SSH_CONNECTION_TEST_AUTHORIZED_KEYS_EXPLANATION)
            .wrap(true)
            .xalign(0.0)
            .build();
        let trust_entry = troubleshooting
            .as_ref()
            .map(|(entry, _)| entry.as_str())
            .unwrap_or_default();
        let buffer = gtk4::TextBuffer::builder().text(trust_entry).build();
        let entry = gtk4::TextView::builder()
            .buffer(&buffer)
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .wrap_mode(gtk4::WrapMode::Char)
            .height_request(96)
            .left_margin(8)
            .right_margin(8)
            .top_margin(8)
            .bottom_margin(8)
            .build();
        let entry_frame = gtk4::ScrolledWindow::builder()
            .has_frame(true)
            .min_content_height(96)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .child(&entry)
            .build();
        let copy = gtk4::Button::builder()
            .label(ui_text::SSH_CONNECTION_TEST_COPY_ENTRY)
            .halign(gtk4::Align::End)
            .build();
        let server_heading = gtk4::Label::builder()
            .label(ui_text::SSH_CONNECTION_TEST_SERVER_HEADING)
            .xalign(0.0)
            .margin_top(6)
            .build();
        server_heading.add_css_class("heading");
        let server_explanation = gtk4::Label::builder()
            .label(
                troubleshooting
                    .as_ref()
                    .map(|(_, configuration)| configuration.as_str())
                    .unwrap_or_default(),
            )
            .wrap(true)
            .xalign(0.0)
            .build();
        let details = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .spacing(8)
            .accessible_role(gtk4::AccessibleRole::Group)
            .build();
        details.update_property(&[gtk4::accessible::Property::Label(
            ui_text::SSH_CONNECTION_TEST_REJECTION_GUIDANCE,
        )]);
        details.append(&authorized_keys_heading);
        details.append(&authorized_keys_explanation);
        details.append(&entry_frame);
        details.append(&copy);
        details.append(&server_heading);
        details.append(&server_explanation);
        details.set_visible(troubleshooting.is_some());
        let toast_overlay = ToastOverlay::new();
        toast_overlay.widget().set_child(Some(&details));
        let dialog = crate::app::ui::message_dialog_builder()
            .transient_for(parent)
            .default_width(600)
            .heading(heading)
            .body(body)
            .extra_child(toast_overlay.widget())
            .close_response("close")
            .build();
        dialog.add_response("close", ui_text::CLOSE);
        let on_copy = std::rc::Rc::clone(&on_event);
        let on_toast = toast_overlay.event_target();
        copy.connect_clicked(move |_| {
            if let Some(toast) = on_copy(SshConnectionTestEvent::ResultCopyRequested) {
                on_toast.send(toast);
            }
        });
        dialog.connect_response(None, move |dialog, _| {
            on_event(SshConnectionTestEvent::ResultClosed);
            dialog.close();
        });
        dialog.present();
    }
}
