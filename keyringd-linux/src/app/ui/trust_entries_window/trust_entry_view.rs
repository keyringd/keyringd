use keyringd_ui_core::runtime::UiEventTarget;
use keyringd_ui_core::trust_entries_window::TrustEntryViewProps;

use gtk4::prelude::{AccessibleExtManual, BoxExt, ButtonExt, WidgetExt};

use keyringd_ui_core::trust_entries_window::TrustEntriesEvent;

pub(in crate::app::ui) struct TrustEntryView;

impl TrustEntryView {
    pub(in crate::app::ui) fn render(
        props: TrustEntryViewProps,
        on_event: UiEventTarget<TrustEntriesEvent>,
    ) -> gtk4::Box {
        let title = gtk4::Label::builder()
            .label(props.kind.title())
            .xalign(0.0)
            .build();
        let subtitle = gtk4::Label::builder()
            .label(props.description)
            .xalign(0.0)
            .wrap(true)
            .build();
        subtitle.add_css_class("dim-label");

        let text = gtk4::Entry::builder()
            .text(props.value.as_str())
            .editable(false)
            .hexpand(true)
            .build();
        text.add_css_class("monospace");

        let copy = gtk4::Button::builder()
            .icon_name("edit-copy-symbolic")
            .tooltip_text(props.kind.copy_label())
            .valign(gtk4::Align::Center)
            .build();
        copy.update_property(&[gtk4::accessible::Property::Label(props.kind.copy_label())]);
        copy.connect_clicked(move |_| {
            on_event.send(TrustEntriesEvent::CopyRequested(props.kind));
        });

        let entry_line = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        entry_line.append(&text);
        entry_line.append(&copy);

        let section = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        section.set_margin_start(12);
        section.set_margin_top(6);
        section.set_margin_bottom(6);
        section.append(&title);
        section.append(&subtitle);
        section.append(&entry_line);
        section
    }
}
