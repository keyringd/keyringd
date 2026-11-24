use keyringd_core::model::{Profile, ProfileId};

pub struct ProfileListProps {
    pub saving: bool,
    pub creation_enabled: bool,
    pub revision: u64,
    pub profiles: Vec<Profile>,
    pub editing: Option<(ProfileId, String)>,
}
