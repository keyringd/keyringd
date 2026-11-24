use super::AppMenuEvent;
use keyringd_core::model::Profile;

pub enum AppMenuCommand {
    RebuildMenu(Vec<Profile>),
    Emit(AppMenuEvent),
    Stop { action: Option<AppMenuEvent> },
}
