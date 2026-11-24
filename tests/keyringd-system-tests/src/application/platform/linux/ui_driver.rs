use std::path::PathBuf;
use std::time::{Duration, Instant};

use super::atspi::{ATSPI_RESPONSE_TIMEOUT, MatchRequirements, Scan, is_removed_accessible};
use anyhow::{Context, Result, bail};
use atspi::Role;
use atspi::State;

use super::Element;
use super::ScreenPoint;
use super::UiLocator;
use super::X11Input;
use crate::application::platform::linux::atspi::AtspiSession;
use crate::environment::linux::Environment;

/// Coordinates UI input, observation deadlines, diagnostics, and screenshots.
/// Feature pages select the product-specific elements and expected states.
#[derive(Clone)]
pub(crate) struct UiDriver {
    atspi: AtspiSession,
    environment: Environment,
}

impl UiDriver {
    pub(crate) fn named(&self, name: &str) -> UiLocator {
        UiLocator::new(self.clone(), name)
    }

    pub(super) async fn read_named<T>(
        &self,
        name: &str,
        role: Option<Role>,
        requirements: MatchRequirements,
        timeout: Duration,
        read: impl AsyncFn(&AtspiSession, &Element, &[State]) -> Result<T>,
    ) -> Result<T> {
        self.wait_until(
            &format!("{role:?} named {name:?} and its value"),
            tokio::time::Instant::now() + timeout,
            async || {
                let mut scan = if let Some(role) = role {
                    self.atspi.find_by_name_and_role(name, role).await?
                } else {
                    self.atspi.find_by_name(name, requirements).await?
                };
                let found = if let Some(element) = scan.found.take() {
                    if let Some(states) = self.atspi.current_element_states(&element).await?
                        && states.contains(&State::Showing)
                    {
                        Some(read(&self.atspi, &element, &states).await?)
                    } else {
                        None
                    }
                } else {
                    None
                };
                Ok(Scan {
                    found,
                    failure: scan.failure,
                })
            },
        )
        .await
    }

    pub(crate) async fn copy_to_clipboard(&self, action: &Element) -> Result<()> {
        let marker = super::X11Clipboard::prepare_copy(&self.environment).await?;
        self.click(action).await?;
        super::X11Clipboard::read_text_after(&self.environment, &marker).await?;
        Ok(())
    }

    pub(crate) async fn press_key_sequence(&self, window_name: &str, keys: &[&str]) -> Result<()> {
        X11Input::press_key_sequence(&self.environment, window_name, keys).await
    }

    pub(crate) async fn focus_with_tab(&self, element: &Element, window_name: &str) -> Result<()> {
        for _ in 0..32 {
            if self
                .atspi
                .element_has_state(element, State::Focused)
                .await?
            {
                return Ok(());
            }
            X11Input::press_key_sequence(&self.environment, window_name, &["Tab"]).await?;
        }
        bail!("element did not receive keyboard focus after traversing the window")
    }

    pub(crate) async fn connect(environment: &Environment) -> Result<Self> {
        Ok(Self {
            atspi: AtspiSession::connect().await?,
            environment: environment.clone(),
        })
    }

    pub(crate) async fn scoped_to_application(&self, element: &Element) -> Result<Self> {
        Ok(Self {
            atspi: self.atspi.scoped_to_application(element).await?,
            environment: self.environment.clone(),
        })
    }

    pub(crate) fn within(&self, root: &Element) -> Self {
        Self {
            atspi: self.atspi.within(root),
            environment: self.environment.clone(),
        }
    }

    pub(crate) fn application(&self) -> Self {
        Self {
            atspi: self.atspi.application(),
            environment: self.environment.clone(),
        }
    }

    pub(crate) async fn wait_for_group(&self, name: &str, timeout: Duration) -> Result<Self> {
        let root = self
            .wait_for_named_role(name, atspi::Role::Grouping, timeout)
            .await?;
        Ok(self.within(&root))
    }

    pub(crate) async fn preceding_sibling_names_with_role(
        &self,
        name: &str,
        role: atspi::Role,
    ) -> Result<Vec<String>> {
        self.read_named(
            name,
            None,
            MatchRequirements::VISIBLE,
            Duration::from_secs(10),
            async |session, element, _| {
                session
                    .preceding_sibling_names_with_role(element, role)
                    .await
            },
        )
        .await
    }

    pub(crate) async fn visible_names_with_role(&self, role: atspi::Role) -> Result<Vec<String>> {
        self.atspi.visible_names_with_role(role).await
    }

    pub(crate) async fn visible_texts_with_role(&self, role: atspi::Role) -> Result<Vec<String>> {
        self.atspi.visible_texts_with_role(role).await
    }

    pub(crate) async fn clipboard_text(&self) -> Result<String> {
        super::X11Clipboard::read_text(&self.environment).await
    }

    pub(crate) async fn containing_row(&self, name: &str) -> Result<Self> {
        let row = self
            .read_named(
                name,
                None,
                MatchRequirements::VISIBLE,
                Duration::from_secs(10),
                async |session, label, _| {
                    session
                        .ancestor_with_role(label, atspi::Role::ListItem)
                        .await
                },
            )
            .await?;
        Ok(self.within(&row))
    }

    pub(crate) async fn wait_for_text_entry(
        &self,
        name: &str,
        timeout: Duration,
    ) -> Result<Element> {
        // A normal AdwEntryRow maps to AT-SPI Text; secret entries use PasswordText.
        self.wait_for_named_role(name, atspi::Role::Text, timeout)
            .await
    }

    pub(crate) async fn wait_for_password_entry(
        &self,
        name: &str,
        timeout: Duration,
    ) -> Result<Element> {
        self.wait_for_named_role(name, atspi::Role::PasswordText, timeout)
            .await
    }

    pub(crate) async fn capture(&self) -> Result<PathBuf> {
        super::screenshot::capture(&self.atspi.application(), &self.environment).await
    }

    pub(crate) async fn read_hybrid_qr(&self, previous: Option<&str>) -> Result<(String, PathBuf)> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match super::screenshot::read_hybrid_qr(&self.atspi.application(), &self.environment)
                .await
            {
                Ok((uri, image)) if previous != Some(uri.as_str()) => return Ok((uri, image)),
                Ok((_, image)) if Instant::now() >= deadline => {
                    bail!(
                        "Hybrid QR still belongs to the previous step; image: {}",
                        image.display()
                    );
                }
                Ok(_) => tokio::time::sleep(Duration::from_millis(100)).await,
                Err(error) if Instant::now() >= deadline => {
                    return Err(error.context("Hybrid QR did not become readable"));
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
            }
        }
    }

    pub(crate) async fn click_then_press_key(&self, element: &Element, key: &str) -> Result<()> {
        X11Input::click_then_press_key(
            &self.environment,
            self.atspi.element_extents(element).await?.center(),
            key,
        )
        .await
    }

    pub(crate) async fn drag(&self, source: &Element, target: ScreenPoint) -> Result<()> {
        X11Input::drag(
            &self.environment,
            self.atspi.element_extents(source).await?.center(),
            target,
        )
        .await
    }

    pub(crate) async fn wait_for_toggle_button(
        &self,
        name: &str,
        timeout: Duration,
    ) -> Result<Element> {
        self.wait_for_named_role(name, atspi::Role::ToggleButton, timeout)
            .await
    }

    pub(crate) async fn visible_password_entry(&self, name: &str) -> Result<Option<Element>> {
        self.atspi.visible_password_entry(name).await
    }

    pub(crate) async fn click(&self, element: &Element) -> Result<()> {
        self.atspi.click(element).await
    }

    pub(crate) async fn replace_text(&self, element: &Element, value: &str) -> Result<()> {
        self.atspi.replace_text(element, value).await
    }
}

impl UiDriver {
    pub(crate) async fn wait_for_dialog_body(
        &self,
        prefix: &str,
        timeout: Duration,
    ) -> Result<(Self, String)> {
        self.wait_until(
            &format!("dialog body starting with {prefix:?}"),
            tokio::time::Instant::now() + timeout,
            async || {
                let mut scan = self
                    .scan_with_states(
                        self.atspi
                            .find_name_matching(|name| name.starts_with(prefix))
                            .await?,
                        &[State::Showing],
                    )
                    .await?;
                let found = if let Some(body) = scan.found.take() {
                    let name = self.atspi.element_name(&body).await?;
                    Some((self.within(&self.atspi.dialog_ancestor(&body).await?), name))
                } else {
                    None
                };
                Ok(Scan {
                    found,
                    failure: scan.failure,
                })
            },
        )
        .await
    }

    pub(crate) async fn visible_name_starting_with(&self, prefix: &str) -> Result<bool> {
        let scan = self
            .scan_with_states(
                self.atspi
                    .find_name_matching(|name| name.starts_with(prefix))
                    .await?,
                &[State::Showing],
            )
            .await?;
        if scan.found.is_some() {
            return Ok(true);
        }
        if let Some(error) = scan.failure {
            return Err(error.context(format!("could not observe name starting with {prefix:?}")));
        }
        Ok(false)
    }

    pub(crate) async fn visible_name_present(&self, name: &str) -> Result<bool> {
        let scan = self
            .atspi
            .find_by_name(name, MatchRequirements::VISIBLE)
            .await?;
        if scan.found.is_some() {
            return Ok(true);
        }
        if let Some(error) = scan.failure
            && !is_removed_accessible(&error)
        {
            return Err(error.context(format!("could not determine whether {name:?} is visible")));
        }
        Ok(false)
    }

    pub(crate) async fn visible_action_enabled(&self, name: &str) -> Result<Option<bool>> {
        let scan = self
            .atspi
            .find_by_name(name, MatchRequirements::VISIBLE_ACTION)
            .await?;
        if let Some(element) = scan.found {
            return Ok(self
                .atspi
                .current_element_states(&element)
                .await?
                .map(|states| states.contains(&State::Sensitive)));
        }
        if let Some(error) = scan.failure
            && !is_removed_accessible(&error)
        {
            return Err(error.context(format!("could not observe action {name:?}")));
        }
        Ok(None)
    }

    pub(crate) async fn wait_for_visible_name(
        &self,
        name: &str,
        timeout: Duration,
    ) -> Result<Element> {
        Ok(self
            .wait_for_any(&[name], timeout, MatchRequirements::VISIBLE)
            .await?
            .1)
    }

    pub(crate) async fn wait_for_visible_action(
        &self,
        name: &str,
        timeout: Duration,
    ) -> Result<Element> {
        self.wait_for_any_visible_action(&[name], timeout).await
    }

    pub(crate) async fn wait_for_any_visible_action(
        &self,
        names: &[&str],
        timeout: Duration,
    ) -> Result<Element> {
        Ok(self
            .wait_for_any(names, timeout, MatchRequirements::VISIBLE_ACTION)
            .await?
            .1)
    }

    pub(crate) async fn action_status(&self, element: &Element) -> Result<(String, bool)> {
        let states = self
            .atspi
            .current_element_states(element)
            .await?
            .context("the action disappeared before its state could be read")?;
        Ok((
            self.atspi.element_name(element).await?,
            states.contains(&State::Sensitive),
        ))
    }

    pub(crate) async fn wait_for_enabled_action(
        &self,
        name: &str,
        timeout: Duration,
    ) -> Result<Element> {
        self.wait_until(
            &format!("enabled action {name:?}"),
            tokio::time::Instant::now() + timeout,
            async || {
                self.scan_with_states(
                    self.atspi
                        .find_by_name(name, MatchRequirements::VISIBLE_ACTION)
                        .await?,
                    &[State::Showing, State::Sensitive],
                )
                .await
            },
        )
        .await
    }

    pub(crate) async fn wait_for_named_role(
        &self,
        name: &str,
        role: Role,
        timeout: Duration,
    ) -> Result<Element> {
        let required = if matches!(role, Role::Text | Role::PasswordText) {
            &[State::Showing, State::Sensitive][..]
        } else {
            &[State::Showing][..]
        };
        self.wait_until(
            &format!("visible {role:?} named {name:?}"),
            tokio::time::Instant::now() + timeout,
            async || {
                self.scan_with_states(
                    self.atspi.find_by_name_and_role(name, role).await?,
                    required,
                )
                .await
            },
        )
        .await
    }

    pub(crate) async fn wait_for_window(&self, title: &str, timeout: Duration) -> Result<Element> {
        self.wait_until(
            &format!("window titled {title:?}"),
            tokio::time::Instant::now() + timeout,
            async || {
                self.scan_with_states(
                    self.atspi
                        .find_by_name_and_roles(
                            title,
                            &[Role::Frame, Role::Dialog, Role::Alert, Role::Window],
                        )
                        .await?,
                    &[State::Showing],
                )
                .await
            },
        )
        .await
    }

    pub(crate) async fn wait_for_absence(&self, name: &str, timeout: Duration) -> Result<()> {
        self.wait_until_absent(name, timeout, MatchRequirements::ANY)
            .await
    }

    pub(crate) async fn wait_for_visible_absence(
        &self,
        name: &str,
        timeout: Duration,
    ) -> Result<()> {
        self.wait_until_absent(name, timeout, MatchRequirements::VISIBLE)
            .await
    }

    async fn wait_until_absent(
        &self,
        name: &str,
        timeout: Duration,
        requirements: MatchRequirements,
    ) -> Result<()> {
        self.wait_until(
            &format!("{name:?} to be absent (showing={})", requirements.showing),
            tokio::time::Instant::now() + timeout,
            async || {
                let scan = self.atspi.find_by_name(name, requirements).await?;
                Ok(Scan {
                    found: (scan.found.is_none() && scan.failure.is_none()).then_some(()),
                    failure: scan.failure,
                })
            },
        )
        .await
    }

    pub(crate) async fn wait_for_radio_checked(
        &self,
        name: &str,
        timeout: Duration,
    ) -> Result<Element> {
        self.wait_until(
            &format!("radio button {name:?} to be checked"),
            tokio::time::Instant::now() + timeout,
            async || {
                self.scan_with_states(
                    self.atspi
                        .find_by_name_and_role(name, Role::RadioButton)
                        .await?,
                    &[State::Checked],
                )
                .await
            },
        )
        .await
    }

    pub(crate) async fn select_radio_button(&self, name: &str, timeout: Duration) -> Result<()> {
        let deadline = tokio::time::Instant::now() + timeout;
        let (element, checked) = self
            .read_named(
                name,
                Some(Role::RadioButton),
                MatchRequirements::VISIBLE,
                timeout,
                async |_, element, states| Ok((element.clone(), states.contains(&State::Checked))),
            )
            .await?;
        tokio::time::timeout_at(deadline, async {
            if checked {
                return Ok(());
            }
            self.atspi
                .click(&element)
                .await
                .context("failed to invoke the radio button's AT-SPI action")?;
            self.wait_until(
                &format!("radio button {name:?} to become checked"),
                deadline,
                async || {
                    self.scan_with_states(
                        self.atspi
                            .find_by_name_and_role(name, Role::RadioButton)
                            .await?,
                        &[State::Checked],
                    )
                    .await
                },
            )
            .await?;
            Ok(())
        })
        .await
        .with_context(|| format!("timed out selecting radio button {name:?}"))?
    }

    pub(crate) async fn wait_for_dialog_action(
        &self,
        name: &str,
        timeout: Duration,
    ) -> Result<(Element, Element)> {
        self.wait_until(
            &format!("dialog action {name:?}"),
            tokio::time::Instant::now() + timeout,
            async || {
                let mut scan = self
                    .scan_with_states(
                        self.atspi
                            .find_by_name(name, MatchRequirements::VISIBLE_ACTION)
                            .await?,
                        &[State::Showing],
                    )
                    .await?;
                let found = if let Some(action) = scan.found.take() {
                    Some((self.atspi.dialog_ancestor(&action).await?, action))
                } else {
                    None
                };
                Ok(Scan {
                    found,
                    failure: scan.failure,
                })
            },
        )
        .await
    }

    pub(crate) async fn wait_for_action_within(
        &self,
        root: &Element,
        name: &str,
        timeout: Duration,
    ) -> Result<Element> {
        self.wait_until(
            &format!("action {name:?} within accessible container"),
            tokio::time::Instant::now() + timeout,
            async || self.atspi.find_action_within(root, name).await,
        )
        .await
    }

    pub(crate) async fn wait_for_dialog(
        &self,
        heading: &str,
        timeout: Duration,
    ) -> Result<Element> {
        self.wait_until(
            &format!("dialog headed {heading:?}"),
            tokio::time::Instant::now() + timeout,
            async || {
                let mut scan = self
                    .scan_with_states(
                        self.atspi.find_by_name_breadth_first(heading).await?,
                        &[State::Showing],
                    )
                    .await?;
                let found = if let Some(heading) = scan.found.take() {
                    Some(self.atspi.dialog_ancestor(&heading).await?)
                } else {
                    None
                };
                Ok(Scan {
                    found,
                    failure: scan.failure,
                })
            },
        )
        .await
    }

    pub(crate) async fn wait_for_any_visible_name(
        &self,
        names: &[&str],
        timeout: Duration,
    ) -> Result<(usize, Element)> {
        self.wait_for_any(names, timeout, MatchRequirements::VISIBLE)
            .await
    }

    async fn wait_for_any(
        &self,
        names: &[&str],
        timeout: Duration,
        requirements: MatchRequirements,
    ) -> Result<(usize, Element)> {
        self.wait_until(
            &format!(
                "accessible name in {names:?}, action={}",
                requirements.action
            ),
            tokio::time::Instant::now() + timeout,
            async || {
                let mut scan = self.atspi.find_by_any_name(names, requirements).await?;
                if let Some((_, element)) = &scan.found
                    && !self
                        .atspi
                        .current_element_states(element)
                        .await?
                        .is_some_and(|states| states.contains(&State::Showing))
                {
                    scan.found = None;
                }
                Ok(scan)
            },
        )
        .await
    }

    async fn scan_with_states(
        &self,
        mut scan: Scan<Element>,
        required: &[State],
    ) -> Result<Scan<Element>> {
        if let Some(element) = &scan.found
            && !self
                .atspi
                .current_element_states(element)
                .await?
                .is_some_and(|states| required.iter().all(|state| states.contains(state)))
        {
            scan.found = None;
        }
        Ok(scan)
    }

    /// Retries observations within one deadline; actions are never replayed.
    async fn wait_until<T>(
        &self,
        description: &str,
        deadline: tokio::time::Instant,
        mut observe: impl AsyncFnMut() -> Result<Scan<T>>,
    ) -> Result<T> {
        let mut last_failure = None;
        loop {
            let mut removed = false;
            match tokio::time::timeout_at(deadline, observe()).await {
                Ok(Ok(scan)) => {
                    if let Some(found) = scan.found {
                        return Ok(found);
                    }
                    if scan.failure.is_some() {
                        removed = scan.failure.as_ref().is_some_and(is_removed_accessible);
                        last_failure = scan.failure;
                    }
                }
                Ok(Err(error)) if is_removed_accessible(&error) => {
                    removed = true;
                    last_failure = Some(error);
                }
                Ok(Err(error)) => {
                    return Err(error.context(format!("could not observe {description}")));
                }
                Err(_) => break,
            }
            if removed {
                self.atspi.invalidate_tree().await;
            }
            if tokio::time::Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep_until(
                deadline.min(tokio::time::Instant::now() + Duration::from_millis(20)),
            )
            .await;
        }
        // Waiting has ended. Failure diagnostics have a separate, fixed request
        // budget, so an unresponsive UI cannot block reporting indefinitely.
        let diagnostics = tokio::time::timeout(ATSPI_RESPONSE_TIMEOUT, async {
            let windows = self.atspi.window_summary().await?;
            let choices = self
                .atspi
                .role_element_summary(&[Role::CheckBox, Role::RadioButton])
                .await?;
            let tree = self.atspi.tree_summary().await?;
            Ok::<_, anyhow::Error>(format!(
                "open windows:\n{windows}\ncheck/radio elements:\n{choices}\nAT-SPI tree:\n{tree}"
            ))
        })
        .await;
        let diagnostics = match diagnostics {
            Ok(Ok(details)) => details,
            Ok(Err(error)) => format!("diagnostics failed: {error:#}"),
            Err(_) => "diagnostics timed out".to_owned(),
        };
        bail!(
            "timed out waiting for {description}; last scan error: {last_failure:?}; {diagnostics}"
        )
    }
}
