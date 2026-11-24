use gtk4::gio;
use gtk4::gio::prelude::ActionMapExt;
use gtk4::glib::object::ObjectExt;
use gtk4::prelude::{AccessibleExtManual, CheckButtonExt, WidgetExt};

#[derive(Clone)]
pub(in crate::app::ui) struct RadioButtonView {
    widget: gtk4::CheckButton,
}

impl RadioButtonView {
    pub(in crate::app::ui) fn new(label: &str) -> Self {
        let widget = gtk4::CheckButton::new();
        widget.update_property(&[gtk4::accessible::Property::Label(label)]);
        let actions = gio::SimpleActionGroup::new();
        let select = gio::SimpleAction::new("select", None);
        let weak_widget = widget.downgrade();
        select.connect_activate(move |_, _| {
            if let Some(widget) = weak_widget.upgrade() {
                widget.set_active(true);
            }
        });
        actions.add_action(&select);
        widget.insert_action_group("radio", Some(&actions));
        Self { widget }
    }

    pub(in crate::app::ui) fn widget(&self) -> &gtk4::CheckButton {
        &self.widget
    }
}
