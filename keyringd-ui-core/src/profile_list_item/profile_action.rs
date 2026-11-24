use super::ProfileListItemEvent;

#[derive(Clone)]
pub struct ProfileAction {
    pub label: &'static str,
    pub identifier: &'static str,
    pub enabled: bool,
    pub separator_before: bool,
    event: ProfileListItemEvent,
}

impl ProfileAction {
    pub(super) fn new(
        label: &'static str,
        identifier: &'static str,
        enabled: bool,
        separator_before: bool,
        event: ProfileListItemEvent,
    ) -> Self {
        Self {
            label,
            identifier,
            enabled,
            separator_before,
            event,
        }
    }
    pub fn activate(&self) -> Option<ProfileListItemEvent> {
        self.enabled.then(|| self.event.clone())
    }
}
