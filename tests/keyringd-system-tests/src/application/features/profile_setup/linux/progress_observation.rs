use crate::application::features::profile_setup::ProfileSetupProgress;
use crate::application::platform::linux::UiDriver;
use anyhow::Result;

pub(super) async fn observe_progress(session: &UiDriver) -> Result<ProfileSetupProgress> {
    let mut texts = session.visible_names_with_role(atspi::Role::Label).await?;
    texts.extend(
        session
            .visible_names_with_role(atspi::Role::RadioButton)
            .await?,
    );
    texts.extend(
        session
            .visible_names_with_role(atspi::Role::Grouping)
            .await?,
    );
    // These identify the displayed page header; return the actual label value.
    let heading = texts
        .iter()
        .find(|text| {
            text.starts_with("Step ")
                || text.as_str() == "Passkey Not Found"
                || text.as_str() == "SSH Profile Created"
        })
        .cloned()
        .unwrap_or_default();
    Ok(ProfileSetupProgress {
        heading,
        security_key_enabled: session.visible_action_enabled("Use security key…").await?,
        continue_enabled: session.visible_action_enabled("Continue").await?,
        pin_visible: session
            .application()
            .visible_name_present("Enter your security key PIN")
            .await?
            || session
                .application()
                .visible_name_present("Set your security key PIN")
                .await?,
        texts,
    })
}
