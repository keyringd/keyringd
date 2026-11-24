use super::MenuBarComponent;
use keyringd_ui_core::app_menu::{AppMenuCommand, AppMenuEvent};
use objc2::MainThreadMarker;

pub(super) struct MenuBarCommandHandler;
impl MenuBarCommandHandler {
    pub(super) fn handle(
        command: AppMenuCommand,
        component: &mut MenuBarComponent,
        mtm: MainThreadMarker,
    ) -> Option<AppMenuEvent> {
        match command {
            AppMenuCommand::RebuildMenu(profiles) => {
                component.render(&profiles, mtm);
                None
            }
            AppMenuCommand::Emit(action) => Some(action),
            AppMenuCommand::Stop { action } => {
                component.remove();
                action
            }
        }
    }
}
