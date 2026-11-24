use keyringd_ui_core::ComponentModel;

#[derive(Default)]
pub(in crate::app::ui::profile_setup_wizard) struct SelectionState {
    pub(in crate::app::ui::profile_setup_wizard) selected_index: usize,
}

impl ComponentModel for SelectionState {
    type Event = usize;
    type Command = ();

    fn update(&mut self, index: usize) -> Option<()> {
        self.selected_index = index;
        None
    }
}
