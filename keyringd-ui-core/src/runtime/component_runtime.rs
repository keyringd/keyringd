use crate::ComponentModel;
use std::cell::{Ref, RefCell};

pub struct ComponentRuntime<Model> {
    model: RefCell<Model>,
}

impl<Model: ComponentModel> ComponentRuntime<Model> {
    pub fn new(model: Model) -> Self {
        Self {
            model: RefCell::new(model),
        }
    }
    pub fn dispatch(&self, event: Model::Event) -> Option<Model::Command> {
        self.model.borrow_mut().update(event)
    }
    pub fn dispatch_and_render(
        &self,
        event: Model::Event,
        render: impl FnOnce(&Model),
    ) -> Option<Model::Command> {
        let command = self.model.borrow_mut().update(event);
        render(&self.model.borrow());
        command
    }
    pub fn reset(&self, model: Model, render: impl FnOnce(&Model)) {
        *self.model.borrow_mut() = model;
        render(&self.model.borrow());
    }
    pub fn state(&self) -> Ref<'_, Model> {
        self.model.borrow()
    }
}
