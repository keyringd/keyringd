use super::{
    LinuxCompletionPage, LinuxPasskeyNotFoundPage, LinuxPasskeySelectionPage, LinuxProfileFormPage,
    LinuxProfileSetupInteractionPage,
};
use crate::application::platform::linux::{ELEMENT_TIMEOUT, KEY_OPERATION_TIMEOUT, UiDriver};
use anyhow::Result;
use tokio::runtime::Runtime;

pub struct LinuxProfileSetupWizard<'a> {
    pub(crate) runtime: &'a Runtime,
    pub(crate) session: UiDriver,
}

impl super::super::ProfileSetupWizardDriver for LinuxProfileSetupWizard<'_> {
    type ProfileFormPage<'a>
        = LinuxProfileFormPage<'a>
    where
        Self: 'a;
    type PasskeySelectionPage<'a>
        = LinuxPasskeySelectionPage<'a>
    where
        Self: 'a;
    type PasskeyNotFoundPage<'a>
        = LinuxPasskeyNotFoundPage<'a>
    where
        Self: 'a;
    type InteractionPage<'a>
        = LinuxProfileSetupInteractionPage<'a>
    where
        Self: 'a;
    type CompletionPage<'a>
        = LinuxCompletionPage<'a>
    where
        Self: 'a;

    fn observe(&mut self) -> Result<super::super::ProfileSetupProgress> {
        self.runtime
            .block_on(super::progress_observation::observe_progress(&self.session))
    }

    fn back(&mut self) -> Result<()> {
        self.runtime.block_on(async {
            let back = self
                .session
                .wait_for_visible_action("Back", ELEMENT_TIMEOUT)
                .await?;
            self.session.click(&back).await?;
            self.session
                .wait_for_text_entry("Profile name", ELEMENT_TIMEOUT)
                .await?;
            Ok(())
        })
    }

    fn profile_form_page(&mut self) -> Result<Self::ProfileFormPage<'_>> {
        self.runtime.block_on(
            self.session
                .wait_for_text_entry("Profile name", ELEMENT_TIMEOUT),
        )?;
        Ok(LinuxProfileFormPage {
            runtime: self.runtime,
            session: self.session.clone(),
        })
    }

    fn passkey_selection_page(&mut self) -> Result<Self::PasskeySelectionPage<'_>> {
        let session = self.runtime.block_on(
            self.session
                .wait_for_group("Passkey choices", KEY_OPERATION_TIMEOUT),
        )?;
        Ok(LinuxPasskeySelectionPage {
            runtime: self.runtime,
            session,
        })
    }

    fn passkey_not_found_page(&mut self) -> Result<Self::PasskeyNotFoundPage<'_>> {
        self.runtime.block_on(
            self.session
                .wait_for_visible_name("Passkey Not Found", ELEMENT_TIMEOUT),
        )?;
        Ok(LinuxPasskeyNotFoundPage {
            runtime: self.runtime,
            session: self.session.clone(),
        })
    }

    fn interaction_page(&mut self) -> Result<Self::InteractionPage<'_>> {
        self.runtime.block_on(
            self.session
                .wait_for_group("Passkey setup interaction", ELEMENT_TIMEOUT),
        )?;
        Ok(LinuxProfileSetupInteractionPage {
            runtime: self.runtime,
            session: self.session.clone(),
        })
    }

    fn completion_page(&mut self) -> Result<Self::CompletionPage<'_>> {
        let session = self.runtime.block_on(
            self.session
                .wait_for_group("Profile creation completed", KEY_OPERATION_TIMEOUT),
        )?;
        Ok(LinuxCompletionPage {
            runtime: self.runtime,
            session,
        })
    }
}
