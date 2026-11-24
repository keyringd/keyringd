use keyringd_core::model::ProfileId;

/// An application action requested from a tray or menu bar.
#[derive(Clone, Debug)]
pub enum AppMenuEvent {
    ShowSshClientConfiguration,
    OpenProfiles,
    AddProfile,
    SetProfileEnabled { id: ProfileId, enabled: bool },
    ShowProfileDetails(ProfileId),
    ShowTrustEntries(ProfileId),
    TestConnection(ProfileId),
    ShowAbout,
    Quit,
}
