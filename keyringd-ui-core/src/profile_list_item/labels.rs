use crate::text;
pub fn enable_action(currently_enabled: bool) -> &'static str {
    if currently_enabled {
        text::DISABLE
    } else {
        text::ENABLE
    }
}

pub fn actions_for(name: &str) -> String {
    format!("Profile actions for {name}")
}
pub fn enable_profile(name: &str) -> String {
    format!("Enable profile {name}")
}
pub fn drag_profile(name: &str) -> String {
    format!("Drag {name} to reorder")
}
pub fn profile_name(name: &str) -> String {
    format!("Profile name for {name}")
}

pub fn actions_menu_for(name: &str) -> String {
    format!("Profile actions menu for {name}")
}
