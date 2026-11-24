use super::HEADING;
use crate::application::features::profile_setup::PasskeySelectionPageDriver;
use crate::application::platform::macos::UiDriver;
use anyhow::{Context, Result, ensure};
const CREDENTIAL_CONTINUE: &str = "profile-setup.credential-continue";
pub struct PasskeySelectionPage<'a> {
    pub(super) driver: UiDriver<'a>,
}
impl PasskeySelectionPageDriver for PasskeySelectionPage<'_> {
    fn candidate_labels(&self) -> Result<Vec<String>> {
        self.driver.wait_value(HEADING, "Choose Passkey")?;
        self.driver
            .wait_until(CREDENTIAL_CONTINUE, "to allow selection", |element| {
                element.enabled == Some(true)
            })?;
        Ok(self
            .visible_candidates()?
            .into_iter()
            .map(|(_, name)| name)
            .collect())
    }
    fn select_passkey(&self, name: &str) -> Result<()> {
        self.driver.wait_value(HEADING, "Choose Passkey")?;
        let candidates = self.visible_candidates()?;
        let matches: Vec<_> = candidates
            .iter()
            .filter(|(_, title)| title == name)
            .collect();
        ensure!(
            matches.len() == 1,
            "expected one Passkey named {name:?}: {candidates:?}"
        );
        let id = &matches[0].0;
        let row = id
            .strip_prefix("profile-setup.credential.")
            .context("Passkey candidate has no row index")?
            .parse()?;
        self.driver
            .select_row("profile-setup.credential-selection", row)?;
        self.driver
            .wait_until(CREDENTIAL_CONTINUE, "to allow continuing", |element| {
                element.enabled == Some(true)
            })?;
        Ok(())
    }

    fn continue_recovery(&self) -> Result<()> {
        self.driver.press(CREDENTIAL_CONTINUE)
    }
}

impl PasskeySelectionPage<'_> {
    fn visible_candidates(&self) -> Result<Vec<(String, String)>> {
        let mut candidates = Vec::new();
        for id in self.driver.identifiers("profile-setup.credential.")? {
            if let Some(element) = self.driver.find_element(&id)?
                && element.hidden != Some(true)
            {
                ensure!(
                    element.role.as_deref() == Some("AXStaticText"),
                    "Passkey candidate is not a table label: {element:?}"
                );
                candidates.push((
                    id,
                    element
                        .value
                        .context("Passkey candidate has no displayed name")?,
                ));
            }
        }
        Ok(candidates)
    }
}
