#[derive(Clone, Copy)]
pub(in crate::application::platform::linux) struct MatchRequirements {
    pub(in crate::application::platform::linux) action: bool,
    pub(in crate::application::platform::linux) showing: bool,
}

impl MatchRequirements {
    pub(in crate::application::platform::linux) const ANY: Self = Self {
        action: false,
        showing: false,
    };
    pub(in crate::application::platform::linux) const VISIBLE: Self = Self {
        action: false,
        showing: true,
    };
    pub(in crate::application::platform::linux) const VISIBLE_ACTION: Self = Self {
        action: true,
        showing: true,
    };
}
