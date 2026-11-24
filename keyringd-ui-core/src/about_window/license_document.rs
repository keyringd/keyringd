#[derive(Clone, Copy)]
pub enum LicenseDocument {
    Application,
    ThirdParty,
}

impl LicenseDocument {
    pub fn title(self) -> &'static str {
        match self {
            Self::Application => "Keyringd License",
            Self::ThirdParty => "Third-party Licenses",
        }
    }
}
