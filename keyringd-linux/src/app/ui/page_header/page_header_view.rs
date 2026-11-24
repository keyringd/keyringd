use gtk4::prelude::{BoxExt, WidgetExt};

pub(in crate::app::ui) struct PageHeaderView {
    pub(in crate::app::ui) root: gtk4::Box,
    pub(in crate::app::ui) heading: gtk4::Label,
    pub(in crate::app::ui) body: gtk4::Label,
}

impl PageHeaderView {
    pub(in crate::app::ui) fn new() -> Self {
        let heading = gtk4::Label::builder()
            .halign(gtk4::Align::Center)
            .wrap(true)
            .build();
        heading.add_css_class("title-1");
        let body = gtk4::Label::builder()
            .halign(gtk4::Align::Center)
            .wrap(true)
            .build();
        body.add_css_class("dim-label");
        let root = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        root.set_margin_bottom(16);
        root.append(&heading);
        root.append(&body);
        Self {
            root,
            heading,
            body,
        }
    }
}
