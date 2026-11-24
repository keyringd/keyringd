use std::collections::HashMap;
use windows_reactor::{Backend, ControlId, ControlKind, Event, EventHandler, Prop, PropValue};

#[derive(Default)]
pub(in crate::app::ui) struct FlyoutBackend {
    pub(in crate::app::ui) next_id: u32,
    pub(in crate::app::ui) handlers: HashMap<ControlId, EventHandler>,
    pub(in crate::app::ui) menus: HashMap<ControlId, EventHandler>,
}

impl Backend for FlyoutBackend {
    fn create(&mut self, _: ControlKind) -> ControlId {
        self.next_id += 1;
        ControlId::new(self.next_id)
    }

    fn set_prop(&mut self, id: ControlId, prop: Prop, _: &PropValue) {
        if prop == Prop::MenuFlyoutItems {
            self.menus.insert(id, self.handlers[&id].clone());
        }
    }

    fn attach_event(&mut self, id: ControlId, event: Event, handler: EventHandler) {
        if event == Event::ItemClicked {
            self.handlers.insert(id, handler);
        }
    }

    fn destroy(&mut self, id: ControlId) {
        self.handlers.remove(&id);
        self.menus.remove(&id);
    }

    fn append_child(&mut self, _: ControlId, _: ControlId) {}
    fn remove_child(&mut self, _: ControlId, _: usize) {}
    fn replace_child(&mut self, _: ControlId, _: usize, _: ControlId) {}
    fn move_child(&mut self, _: ControlId, _: usize, _: usize) {}
    fn insert_child(&mut self, _: ControlId, _: usize, _: ControlId) {}
}
