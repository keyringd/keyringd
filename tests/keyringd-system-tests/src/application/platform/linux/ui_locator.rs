use std::time::Duration;

use anyhow::Result;
use atspi::{Role, State};

use super::atspi::MatchRequirements;
use super::{ScreenRect, UiDriver};

/// Keeps a query, never an accessible object that GTK may replace.
pub(crate) struct UiLocator {
    driver: UiDriver,
    name: String,
    role: Option<Role>,
    requirements: MatchRequirements,
}

impl UiLocator {
    pub(super) fn new(driver: UiDriver, name: &str) -> Self {
        Self {
            driver,
            name: name.to_owned(),
            role: None,
            requirements: MatchRequirements::VISIBLE,
        }
    }

    pub(crate) fn role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }

    pub(crate) fn action(mut self) -> Self {
        self.requirements = MatchRequirements::VISIBLE_ACTION;
        self
    }

    pub(crate) async fn checked(&self, timeout: Duration) -> Result<bool> {
        self.read(timeout, async |_, _, states| {
            Ok(states.contains(&State::Checked))
        })
        .await
    }

    pub(crate) async fn enabled(&self, timeout: Duration) -> Result<bool> {
        self.read(timeout, async |_, _, states| {
            Ok(states.contains(&State::Sensitive))
        })
        .await
    }

    pub(crate) async fn text(&self, timeout: Duration) -> Result<String> {
        self.read(timeout, async |session, element, _| {
            session.read_text(element).await
        })
        .await
    }

    pub(crate) async fn extents(&self, timeout: Duration) -> Result<ScreenRect> {
        self.read(timeout, async |session, element, _| {
            session.element_extents(element).await
        })
        .await
    }

    async fn read<T>(
        &self,
        timeout: Duration,
        read: impl AsyncFn(&super::atspi::AtspiSession, &super::Element, &[State]) -> Result<T>,
    ) -> Result<T> {
        self.driver
            .read_named(&self.name, self.role, self.requirements, timeout, read)
            .await
    }
}
