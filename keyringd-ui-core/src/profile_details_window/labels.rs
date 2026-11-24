use crate::text;
pub fn heading(name: &str) -> String {
    format!("Details for “{name}”")
}
pub fn status(enabled: bool) -> &'static str {
    if enabled {
        text::PROFILE_LIST_ITEM_ENABLED
    } else {
        text::PROFILE_DISABLED
    }
}

pub fn replacement_description(backend: &str) -> String {
    format!("Create a new {backend} key and authorize it with the Passkey.")
}
