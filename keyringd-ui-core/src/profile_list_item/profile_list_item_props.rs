use keyringd_core::model::Profile;

pub struct ProfileListItemProps {
    pub saving: bool,
    pub profile: Profile,
    pub editing_name: Option<String>,
}

impl ProfileListItemProps {
    pub fn subtitle(&self) -> String {
        format!(
            "{} · {}",
            self.profile.device_key().signature_algorithm().as_str(),
            crate::device_key_protection_window::authentication_label(
                self.profile.device_key().local_authentication()
            )
        )
    }
}
