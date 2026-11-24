use std::cell::RefCell;
use std::rc::Rc;
use windows_reactor::Updater;

pub(crate) use keyringd_ui_core::ComponentModel;

pub(crate) fn reduce<Model>(
    set_model: &Updater<Model>,
    event: Model::Event,
) -> (Model, Option<Model::Command>)
where
    Model: ComponentModel + Clone + 'static,
    Model::Event: 'static,
{
    let result = Rc::new(RefCell::new(None));
    let updated_result = Rc::clone(&result);
    // Reactor executes this updater synchronously on the UI thread. Events
    // from the same render apply to the current state, in arrival order.
    set_model.call(move |mut current| {
        let command = current.update(event);
        *updated_result.borrow_mut() = Some((current.clone(), command));
        current
    });
    result.take().expect(
        "the reduced model should be available because the Reactor updater executes synchronously",
    )
}
