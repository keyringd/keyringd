use super::is_removed_accessible;
use super::{ATSPI_RESPONSE_TIMEOUT, ApplicationTree, Traversal, WindowCandidate};

use super::MatchRequirements;
use super::Scan;

use std::future::Future;
use std::sync::Arc;

use anyhow::{Context, Result};
use atspi::AccessibilityConnection;
use atspi::ObjectRefOwned;
use atspi::events::CacheEvents;
use atspi::events::object::{ChildrenChangedEvent, PropertyChangeEvent};
use atspi::proxy::accessible::AccessibleProxy;

use super::super::Element;

fn supported_click_action(actions: &[atspi::Action]) -> Option<usize> {
    actions.iter().position(|candidate| {
        ["click", "press", "activate", "toggle", "radio.select"]
            .iter()
            .any(|name| candidate.name.eq_ignore_ascii_case(name))
    })
}

async fn bounded<T, E>(future: impl Future<Output = std::result::Result<T, E>>) -> Option<T> {
    tokio::time::timeout(ATSPI_RESPONSE_TIMEOUT, future)
        .await
        .ok()?
        .ok()
}

async fn observed<T, E: Into<anyhow::Error>>(
    future: impl Future<Output = std::result::Result<T, E>>,
    failure: &mut Option<anyhow::Error>,
) -> Option<T> {
    match tokio::time::timeout(ATSPI_RESPONSE_TIMEOUT, future).await {
        Ok(Ok(value)) => Some(value),
        Ok(Err(error)) => {
            *failure = Some(error.into());
            None
        }
        Err(_) => {
            *failure = Some(anyhow::anyhow!("AT-SPI request timed out"));
            None
        }
    }
}

/// GetRole's D-Bus contract is an unsigned integer. GTK can expose role values
/// added after atspi-common's closed Rust enum.
/// Read the actual wire value when filtering a traversal by a known role;
/// preserve protocol errors instead of treating them as absent elements.
async fn accessible_role_value(accessible: &AccessibleProxy<'_>) -> Result<u32> {
    let role: u32 = tokio::time::timeout(
        ATSPI_RESPONSE_TIMEOUT,
        accessible.inner().call("GetRole", &()),
    )
    .await
    .context("timed out reading the AT-SPI role value")??;
    Ok(role)
}

/// Shares an AT-SPI connection and tree cache across scoped, single observations.
/// Polling and UI action sequencing belong to `UiDriver`.
#[derive(Clone)]
pub(in crate::application::platform::linux) struct AtspiSession {
    pub(super) connection: AccessibilityConnection,
    pub(super) application_bus_name: Option<String>,
    tree: Option<Arc<ApplicationTree>>,
    pub(super) scope: Option<Element>,
}

impl AtspiSession {
    pub(in crate::application::platform::linux) async fn connect() -> Result<Self> {
        atspi::connection::set_session_accessibility(true)
            .await
            .context("failed to enable session accessibility")?;
        let connection = AccessibilityConnection::new()
            .await
            .context("failed to connect to the AT-SPI bus")?;
        // Subscriptions belong to the shared connection, rather than each
        // application tree created after launching or restarting the process.
        connection
            .add_match_rule::<CacheEvents>()
            .await
            .context("failed to subscribe to AT-SPI cache events")?;
        connection
            .register_event::<ChildrenChangedEvent>()
            .await
            .context("failed to subscribe to AT-SPI child changes")?;
        connection
            .register_event::<PropertyChangeEvent>()
            .await
            .context("failed to subscribe to AT-SPI property changes")?;
        Ok(Self {
            connection,
            application_bus_name: None,
            tree: None,
            scope: None,
        })
    }

    pub(in crate::application::platform::linux) async fn scoped_to_application(
        &self,
        element: &Element,
    ) -> Result<Self> {
        let application_bus_name = element
            .object()
            .name()
            .map(|name| name.as_str().to_owned())
            .context("accessible element has no application bus name")?;
        let tree = match ApplicationTree::start(&self.connection, element.object().clone()).await {
            Ok(tree) => Some(Arc::new(tree)),
            Err(error) => {
                tracing::warn!(%error, "AT-SPI cache unavailable; using live tree traversal");
                None
            }
        };
        Ok(Self {
            connection: self.connection.clone(),
            application_bus_name: Some(application_bus_name),
            tree,
            scope: None,
        })
    }

    pub(in crate::application::platform::linux) fn within(&self, root: &Element) -> Self {
        let mut driver = self.clone();
        driver.scope = Some(root.clone());
        driver
    }

    pub(in crate::application::platform::linux) fn application(&self) -> Self {
        let mut driver = self.clone();
        driver.scope = None;
        driver
    }

    pub(super) async fn root_children(&self) -> Result<Vec<ObjectRefOwned>> {
        if let Some(root) = &self.scope {
            return Ok(vec![root.object().clone()]);
        }
        let registry = AccessibleProxy::builder(self.connection.connection())
            .destination("org.a11y.atspi.Registry")?
            .path("/org/a11y/atspi/accessible/root")?
            .build()
            .await
            .context("failed to create the AT-SPI registry proxy")?;
        tokio::time::timeout(ATSPI_RESPONSE_TIMEOUT, registry.get_children())
            .await
            .context("timed out listing accessible applications")?
            .context("failed to list accessible applications")
    }

    pub(super) fn includes(&self, object: &ObjectRefOwned) -> bool {
        self.application_bus_name.as_deref().is_none_or(|expected| {
            object
                .name()
                .is_some_and(|actual| actual.as_str() == expected)
        })
    }
}
use atspi::proxy::accessible::ObjectRefExt;
use atspi::proxy::action::ActionProxy;
use atspi::proxy::component::ComponentProxy;
use atspi::{CoordType, Role};

use std::collections::VecDeque;

impl AtspiSession {
    pub(in crate::application::platform::linux) async fn role_element_summary(
        &self,
        roles: &[Role],
    ) -> Result<String> {
        let connection = self.connection.connection();
        let mut pending = self.root_children().await?;
        let mut matches = Vec::new();

        while let Some(object) = pending.pop() {
            if object.is_null() || !self.includes(&object) {
                continue;
            }
            let Some(accessible) = bounded(object.as_accessible_proxy(connection)).await else {
                continue;
            };
            let role = bounded(accessible.get_role()).await;
            if role.is_some_and(|role| roles.contains(&role)) {
                let name = bounded(accessible.name()).await;
                let interfaces = bounded(accessible.get_interfaces()).await;
                let action = ActionProxy::builder(connection)
                    .destination(accessible.inner().destination().clone())?
                    .path(accessible.inner().path().clone())?
                    .build()
                    .await?;
                let actions = bounded(action.get_actions()).await;
                let component = ComponentProxy::builder(connection)
                    .destination(accessible.inner().destination().clone())?
                    .path(accessible.inner().path().clone())?
                    .build()
                    .await?;
                let extents = bounded(component.get_extents(CoordType::Screen)).await;
                matches.push(format!(
                    "{object:?}: name={name:?}, role={role:?}, interfaces={interfaces:?}, actions={actions:?}, extents={extents:?}"
                ));
            }
            if let Some(mut children) = bounded(accessible.get_children()).await {
                pending.append(&mut children);
            }
        }

        Ok(if matches.is_empty() {
            "<none>".to_owned()
        } else {
            matches.join("\n")
        })
    }

    pub(in crate::application::platform::linux) async fn tree_summary(&self) -> Result<String> {
        let connection = self.connection.connection();
        let mut pending = self.root_children().await?;
        let mut names = Vec::new();

        while let Some(object) = pending.pop() {
            if object.is_null() || names.len() >= 1_000 {
                continue;
            }
            if !self.includes(&object) {
                continue;
            }
            let Ok(accessible) = object.as_accessible_proxy(connection).await else {
                names.push(format!("{object:?}: <unresolvable>"));
                continue;
            };
            let name = bounded(accessible.name())
                .await
                .unwrap_or_else(|| "<unresponsive>".to_owned());
            names.push(format!("{object:?}: {name:?}"));
            if let Some(mut children) = bounded(accessible.get_children()).await {
                pending.append(&mut children);
            }
        }

        if names.is_empty() {
            Ok("<empty>".to_owned())
        } else {
            names.sort();
            Ok(names.join("\n"))
        }
    }

    pub(in crate::application::platform::linux) async fn window_summary(&self) -> Result<String> {
        let connection = self.connection.connection();
        let applications = self.root_children().await?;
        let mut pending = VecDeque::new();
        for application in applications {
            pending.push_back((application, None));
        }
        let mut windows = Vec::new();

        while let Some((object, application_name)) = pending.pop_front() {
            if object.is_null() {
                continue;
            }
            let Some(accessible) = bounded(object.as_accessible_proxy(connection)).await else {
                continue;
            };
            let role = bounded(accessible.get_role()).await;
            let name = bounded(accessible.name())
                .await
                .unwrap_or_else(|| "<unresponsive>".to_owned());
            let application_name = if role == Some(Role::Application) {
                Some(name.clone())
            } else {
                application_name
            };
            if role.is_some_and(|role| {
                matches!(
                    role,
                    Role::Frame | Role::Dialog | Role::Alert | Role::Window
                )
            }) {
                windows.push(format!(
                    "application={:?}, role={}, title={name:?}",
                    application_name.as_deref().unwrap_or("<unknown>"),
                    role.expect("the role should be present because the preceding is_some_and check matched a window role")
                ));
            }
            if let Some(children) = bounded(accessible.get_children()).await {
                pending.extend(
                    children
                        .into_iter()
                        .map(|child| (child, application_name.clone())),
                );
            }
        }

        if windows.is_empty() {
            Ok("<none>".to_owned())
        } else {
            windows.sort();
            Ok(windows.join("\n"))
        }
    }
}
use crate::application::platform::linux::ScreenRect;
use anyhow::{bail, ensure};
use atspi::State;

fn unique_window(
    windows: &[WindowCandidate],
    predicate: impl Fn(&WindowCandidate) -> bool,
) -> Option<&WindowCandidate> {
    let mut matches = windows.iter().filter(|window| predicate(window));
    let first = matches.next();
    first.filter(|_| matches.next().is_none())
}

impl AtspiSession {
    pub(in crate::application::platform::linux) async fn element_extents(
        &self,
        element: &Element,
    ) -> Result<ScreenRect> {
        let connection = self.connection.connection();
        let mut object = element.object().clone();
        let mut x = 0;
        let mut y = 0;
        let mut size = None;
        loop {
            let accessible = tokio::time::timeout(
                ATSPI_RESPONSE_TIMEOUT,
                object.as_accessible_proxy(connection),
            )
            .await
            .context("timed out resolving accessible element")?
            .context("failed to resolve accessible element")?;
            let role = tokio::time::timeout(ATSPI_RESPONSE_TIMEOUT, accessible.get_role())
                .await
                .context("timed out reading accessible role")?
                .context("failed to read accessible role")?;
            let component = ComponentProxy::builder(connection)
                .destination(accessible.inner().destination().clone())?
                .path(accessible.inner().path().clone())?
                .build()
                .await?;
            let is_top_level = matches!(
                role,
                Role::Frame | Role::Dialog | Role::Alert | Role::Window
            );
            let coordinate_type = if is_top_level {
                CoordType::Screen
            } else {
                CoordType::Parent
            };
            let (local_x, local_y, width, height) = tokio::time::timeout(
                ATSPI_RESPONSE_TIMEOUT,
                component.get_extents(coordinate_type),
            )
            .await
            .context("timed out reading accessible bounds")?
            .context("failed to read accessible bounds")?;
            x += local_x;
            y += local_y;
            size.get_or_insert((width, height));
            if is_top_level {
                break;
            }
            let parent = tokio::time::timeout(ATSPI_RESPONSE_TIMEOUT, accessible.parent())
                .await
                .context("timed out resolving accessible parent")?
                .context("failed to resolve accessible parent")?;
            ensure!(
                !parent.is_null(),
                "accessible element has no top-level window ancestor"
            );
            object = parent;
        }
        let (width, height) = size.context("accessible element exposes no bounds")?;
        ensure!(
            width > 0 && height > 0,
            "accessible element has empty bounds"
        );
        Ok(ScreenRect {
            x,
            y,
            width,
            height,
        })
    }

    pub(in crate::application::platform::linux) async fn capture_extents(
        &self,
    ) -> Result<ScreenRect> {
        let result = if let Some(root) = &self.scope {
            self.element_extents(root).await
        } else {
            self.current_window_extents().await
        };
        if let Err(error) = &result
            && is_removed_accessible(error)
            && let Some(tree) = &self.tree
        {
            tree.invalidate().await;
        }
        result
    }

    pub(in crate::application::platform::linux) async fn current_window_extents(
        &self,
    ) -> Result<ScreenRect> {
        let connection = self.connection.connection();
        let mut pending = self.root_children().await?;
        let mut windows = Vec::new();

        while let Some(object) = pending.pop() {
            if object.is_null() || !self.includes(&object) {
                continue;
            }
            let Ok(accessible) = object.as_accessible_proxy(connection).await else {
                continue;
            };
            let role = accessible.get_role().await.ok();
            if matches!(
                role,
                Some(Role::Frame | Role::Dialog | Role::Alert | Role::Window)
            ) {
                let states = accessible.get_state().await?;
                if !states.iter().any(|state| state == State::Showing) {
                    continue;
                }
                let component = ComponentProxy::builder(connection)
                    .destination(accessible.inner().destination().clone())?
                    .path(accessible.inner().path().clone())?
                    .build()
                    .await?;
                if let Ok(extents @ (_, _, width, height)) =
                    component.get_extents(CoordType::Screen).await
                    && width > 0
                    && height > 0
                {
                    windows.push(WindowCandidate {
                        bounds: extents,
                        active: states.iter().any(|state| state == State::Active),
                        modal: states.iter().any(|state| state == State::Modal),
                        dialog: matches!(role, Some(Role::Dialog | Role::Alert)),
                    });
                }
            }
            if let Ok(mut children) = accessible.get_children().await {
                pending.append(&mut children);
            }
        }

        let selected = unique_window(&windows, |window| window.active && window.modal)
            .or_else(|| unique_window(&windows, |window| window.modal))
            .or_else(|| unique_window(&windows, |window| window.active))
            .or_else(|| (windows.len() == 1).then(|| &windows[0]))
            .or_else(|| unique_window(&windows, |window| window.dialog));
        let Some(selected) = selected else {
            bail!(
                "could not identify one current window from {} visible AT-SPI windows",
                windows.len()
            );
        };
        let (x, y, width, height) = selected.bounds;
        Ok(ScreenRect {
            x,
            y,
            width,
            height,
        })
    }
}

use atspi::proxy::editable_text::EditableTextProxy;
use atspi::proxy::text::TextProxy;

impl AtspiSession {
    pub(in crate::application::platform::linux) async fn click(
        &self,
        element: &Element,
    ) -> Result<()> {
        let connection = self.connection.connection();
        let accessible = element
            .object()
            .as_accessible_proxy(connection)
            .await
            .context("failed to resolve accessible element")?;
        let action = ActionProxy::builder(connection)
            .destination(accessible.inner().destination().clone())?
            .path(accessible.inner().path().clone())?
            .build()
            .await
            .context("element does not expose the AT-SPI Action interface")?;
        let actions = action
            .get_actions()
            .await
            .context("failed to list element actions")?;
        let index = supported_click_action(&actions)
            .with_context(|| format!("element exposes no supported click action: {actions:?}"))?;
        tracing::trace!(
            object = ?element.object(),
            name = ?accessible.name().await.ok(),
            role = ?accessible.get_role().await.ok(),
            states = ?accessible.get_state().await.ok(),
            action = %actions[index].name,
            "invoking AT-SPI action"
        );
        let performed = action
            .do_action(i32::try_from(index)?)
            .await
            .context("failed to invoke element action")?;
        if !performed {
            bail!("AT-SPI declined the action for the selected element");
        }
        Ok(())
    }

    pub(in crate::application::platform::linux) async fn element_name(
        &self,
        element: &Element,
    ) -> Result<String> {
        Ok(element
            .object()
            .as_accessible_proxy(self.connection.connection())
            .await?
            .name()
            .await?)
    }

    pub(in crate::application::platform::linux) async fn read_text(
        &self,
        element: &Element,
    ) -> Result<String> {
        let connection = self.connection.connection();
        let accessible = element.object().as_accessible_proxy(connection).await?;
        let text = TextProxy::builder(connection)
            .destination(accessible.inner().destination().clone())?
            .path(accessible.inner().path().clone())?
            .build()
            .await?;
        Ok(text.get_text(0, -1).await?)
    }

    pub(in crate::application::platform::linux) async fn replace_text(
        &self,
        element: &Element,
        value: &str,
    ) -> Result<()> {
        let connection = self.connection.connection();
        let accessible = element
            .object()
            .as_accessible_proxy(connection)
            .await
            .context("failed to resolve editable element")?;
        let editable = EditableTextProxy::builder(connection)
            .destination(accessible.inner().destination().clone())?
            .path(accessible.inner().path().clone())?
            .build()
            .await
            .context("element does not expose the AT-SPI EditableText interface")?;
        if !editable
            .set_text_contents(value)
            .await
            .context("failed to replace accessible text")?
        {
            bail!("AT-SPI declined replacing text in the selected element");
        }
        Ok(())
    }
}

impl AtspiSession {
    async fn matches_requirements(
        &self,
        accessible: &AccessibleProxy<'_>,
        requirements: MatchRequirements,
        failure: &mut Option<anyhow::Error>,
    ) -> Result<bool> {
        if requirements.action {
            let action = ActionProxy::builder(self.connection.connection())
                .destination(accessible.inner().destination().clone())?
                .path(accessible.inner().path().clone())?
                .build()
                .await?;
            if !observed(action.get_actions(), failure)
                .await
                .is_some_and(|actions| supported_click_action(&actions).is_some())
            {
                return Ok(false);
            }
        }
        Ok(!requirements.showing
            || observed(accessible.get_state(), failure)
                .await
                .is_some_and(|states| states.into_iter().any(|state| state == State::Showing)))
    }

    pub(in crate::application::platform::linux) async fn find_by_name(
        &self,
        name: &str,
        requirements: MatchRequirements,
    ) -> Result<Scan<Element>> {
        let scan = match self.find_by_any_name(&[name], requirements).await {
            Ok(scan) => scan,
            Err(error) if is_removed_accessible(&error) => Scan {
                found: None,
                failure: Some(error),
            },
            Err(error) => return Err(error),
        };
        if scan.failure.as_ref().is_some_and(is_removed_accessible)
            && let Some(tree) = &self.tree
        {
            tree.invalidate().await;
        }
        Ok(scan.map(|(_, element)| element))
    }

    #[tracing::instrument(level = "trace", skip(self, requirements))]
    pub(in crate::application::platform::linux) async fn find_by_any_name(
        &self,
        names: &[&str],
        requirements: MatchRequirements,
    ) -> Result<Scan<(usize, Element)>> {
        if self.scope.is_none()
            && let Some(tree) = &self.tree
            && let Some(items) = tree.reachable(Traversal::DepthFirst).await
        {
            tracing::trace!(candidates = items.len(), "searching cached AT-SPI tree");
            let connection = self.connection.connection();
            let mut first_match = None;
            let mut failure = None;
            for item in items {
                let Some(index) = names
                    .iter()
                    .position(|expected| *expected == item.short_name)
                else {
                    continue;
                };
                let object = item.object;
                let Some(accessible) =
                    observed(object.as_accessible_proxy(connection), &mut failure).await
                else {
                    continue;
                };
                if self
                    .matches_requirements(&accessible, requirements, &mut failure)
                    .await?
                {
                    if index == 0 {
                        return Ok(Scan {
                            found: Some((index, Element::new(object))),
                            failure,
                        });
                    }
                    if first_match
                        .as_ref()
                        .is_none_or(|(previous, _)| index < *previous)
                    {
                        first_match = Some((index, Element::new(object)));
                    }
                }
            }
            return Ok(Scan {
                found: first_match,
                failure,
            });
        }
        tracing::trace!("falling back to live AT-SPI traversal");
        let connection = self.connection.connection();
        let mut pending = self.root_children().await?;
        let mut first_match = None;
        let mut failure = None;

        while let Some(object) = pending.pop() {
            if object.is_null() {
                continue;
            }
            if !self.includes(&object) {
                continue;
            }
            let Some(accessible) =
                observed(object.as_accessible_proxy(connection), &mut failure).await
            else {
                continue;
            };
            let name_index = observed(accessible.name(), &mut failure)
                .await
                .and_then(|actual| names.iter().position(|expected| *expected == actual));
            if let Some(index) = name_index
                && self
                    .matches_requirements(&accessible, requirements, &mut failure)
                    .await?
            {
                if index == 0 {
                    return Ok(Scan {
                        found: Some((index, Element::new(object))),
                        failure,
                    });
                }
                if first_match
                    .as_ref()
                    .is_none_or(|(previous, _)| index < *previous)
                {
                    first_match = Some((index, Element::new(object.clone())));
                }
            }
            if let Some(mut children) = observed(accessible.get_children(), &mut failure).await {
                pending.append(&mut children);
            }
        }
        Ok(Scan {
            found: first_match,
            failure,
        })
    }

    pub(in crate::application::platform::linux) async fn find_by_name_and_role(
        &self,
        name: &str,
        role: Role,
    ) -> Result<Scan<Element>> {
        self.find_by_name_and_roles(name, &[role]).await
    }

    #[tracing::instrument(level = "trace", skip(self, roles))]
    pub(in crate::application::platform::linux) async fn find_by_name_and_roles(
        &self,
        name: &str,
        roles: &[Role],
    ) -> Result<Scan<Element>> {
        if self.scope.is_none()
            && let Some(tree) = &self.tree
            && let Some(items) = tree.reachable(Traversal::DepthFirst).await
        {
            for item in items
                .into_iter()
                .filter(|item| item.short_name == name && roles.contains(&item.role))
            {
                let element = Element::new(item.object);
                if self.current_element_states(&element).await?.is_some() {
                    return Ok(Scan {
                        found: Some(element),
                        failure: None,
                    });
                }
            }
            return Ok(Scan {
                found: None,
                failure: None,
            });
        }
        let connection = self.connection.connection();
        let mut pending = self.root_children().await?;
        let mut failure = None;

        while let Some(object) = pending.pop() {
            if object.is_null() || !self.includes(&object) {
                continue;
            }
            let Some(accessible) =
                observed(object.as_accessible_proxy(connection), &mut failure).await
            else {
                continue;
            };
            if observed(accessible.name(), &mut failure).await.as_deref() == Some(name)
                && observed(accessible.get_role(), &mut failure)
                    .await
                    .is_some_and(|role| roles.contains(&role))
            {
                return Ok(Scan {
                    found: Some(Element::new(object)),
                    failure,
                });
            }
            if let Some(mut children) = observed(accessible.get_children(), &mut failure).await {
                pending.append(&mut children);
            }
        }
        Ok(Scan {
            found: None,
            failure,
        })
    }

    pub(in crate::application::platform::linux) async fn find_action_within(
        &self,
        root: &Element,
        name: &str,
    ) -> Result<Scan<Element>> {
        self.find_action_within_matching(root, |actual| actual == name)
            .await
    }

    async fn find_action_within_matching(
        &self,
        root: &Element,
        matches_name: impl Fn(&str) -> bool,
    ) -> Result<Scan<Element>> {
        let connection = self.connection.connection();
        let mut pending = VecDeque::from([root.object().clone()]);
        let mut failure = None;

        while let Some(object) = pending.pop_front() {
            let Some(accessible) =
                observed(object.as_accessible_proxy(connection), &mut failure).await
            else {
                continue;
            };
            if observed(accessible.name(), &mut failure)
                .await
                .as_deref()
                .is_some_and(&matches_name)
            {
                if !observed(accessible.get_state(), &mut failure)
                    .await
                    .is_some_and(|states| states.contains(State::Showing))
                {
                    continue;
                }
                let action = ActionProxy::builder(connection)
                    .destination(accessible.inner().destination().clone())?
                    .path(accessible.inner().path().clone())?
                    .build()
                    .await?;
                if observed(action.get_actions(), &mut failure)
                    .await
                    .is_some_and(|actions| supported_click_action(&actions).is_some())
                {
                    return Ok(Scan {
                        found: Some(Element::new(object)),
                        failure,
                    });
                }
            }
            if let Some(children) = observed(accessible.get_children(), &mut failure).await {
                pending.extend(children);
            }
        }
        Ok(Scan {
            found: None,
            failure,
        })
    }

    pub(in crate::application::platform::linux) async fn element_has_state(
        &self,
        element: &Element,
        state: State,
    ) -> Result<bool> {
        Ok(self.element_states(element).await?.contains(&state))
    }

    pub(in crate::application::platform::linux) async fn visible_password_entry(
        &self,
        name: &str,
    ) -> Result<Option<Element>> {
        let scan = self.find_by_name_and_role(name, Role::PasswordText).await?;
        if let Some(element) = scan.found
            && let Some(states) = self.current_element_states(&element).await?
            && states.contains(&State::Showing)
            && states.contains(&State::Sensitive)
        {
            return Ok(Some(element));
        }
        if let Some(error) = scan.failure {
            return Err(error.context(format!("could not observe password entry {name:?}")));
        }
        Ok(None)
    }

    /// A destroyed candidate is absent from the current UI, even if a cache
    /// removal event is still pending. Rebuild the cache on the next lookup.
    pub(in crate::application::platform::linux) async fn current_element_states(
        &self,
        element: &Element,
    ) -> Result<Option<Vec<State>>> {
        match self.element_states(element).await {
            Ok(states) if !states.contains(&State::Defunct) => Ok(Some(states)),
            Ok(_) => {
                if let Some(tree) = &self.tree {
                    tree.invalidate().await;
                }
                Ok(None)
            }
            Err(error) if is_removed_accessible(&error) => {
                tracing::trace!(%error, "accessible lookup candidate was removed");
                if let Some(tree) = &self.tree {
                    tree.invalidate().await;
                }
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    pub(super) async fn element_states(&self, element: &Element) -> Result<Vec<State>> {
        let accessible = element
            .object()
            .as_accessible_proxy(self.connection.connection())
            .await
            .context("failed to resolve accessible element")?;
        Ok(accessible
            .get_state()
            .await
            .context("failed to read accessible element state")?
            .into_iter()
            .collect())
    }

    pub(in crate::application::platform::linux) async fn find_by_name_breadth_first(
        &self,
        name: &str,
    ) -> Result<Scan<Element>> {
        self.find_name_matching(|actual| actual == name).await
    }

    pub(in crate::application::platform::linux) async fn find_name_matching(
        &self,
        matches: impl Fn(&str) -> bool,
    ) -> Result<Scan<Element>> {
        if self.scope.is_none()
            && let Some(tree) = &self.tree
            && let Some(items) = tree.reachable(Traversal::BreadthFirst).await
        {
            return Ok(Scan {
                found: items
                    .into_iter()
                    .find(|item| matches(&item.short_name))
                    .map(|item| Element::new(item.object)),
                failure: None,
            });
        }
        let connection = self.connection.connection();
        let mut pending = VecDeque::from(self.root_children().await?);
        let mut failure = None;

        while let Some(object) = pending.pop_front() {
            if object.is_null() || !self.includes(&object) {
                continue;
            }
            let Some(accessible) =
                observed(object.as_accessible_proxy(connection), &mut failure).await
            else {
                continue;
            };
            let accessible_name = observed(accessible.name(), &mut failure).await;
            if accessible_name.as_deref().is_some_and(&matches) {
                return Ok(Scan {
                    found: Some(Element::new(object)),
                    failure,
                });
            }
            if let Some(children) = observed(accessible.get_children(), &mut failure).await {
                pending.extend(children);
            }
        }
        Ok(Scan {
            found: None,
            failure,
        })
    }

    /// MessageDialog places its heading container before the body, and extra
    /// content after it. Observe only the preceding sibling subtrees so extra
    /// children (including transient toasts) cannot become heading candidates.
    pub(in crate::application::platform::linux) async fn preceding_sibling_names_with_role(
        &self,
        element: &Element,
        role: Role,
    ) -> Result<Vec<String>> {
        let connection = self.connection.connection();
        let accessible = element.object().as_accessible_proxy(connection).await?;
        let parent = accessible.parent().await?;
        let parent = parent.as_accessible_proxy(connection).await?;
        let mut names = Vec::new();
        for sibling in parent.get_children().await? {
            if sibling == *element.object() {
                break;
            }
            let mut pending = vec![sibling];
            while let Some(object) = pending.pop() {
                let accessible = object.as_accessible_proxy(connection).await?;
                if accessible_role_value(&accessible).await? == role as u32
                    && accessible.get_state().await?.contains(State::Showing)
                {
                    names.push(accessible.name().await?);
                }
                pending.extend(accessible.get_children().await?);
            }
        }
        Ok(names)
    }

    pub(in crate::application::platform::linux) async fn visible_names_with_role(
        &self,
        role: Role,
    ) -> Result<Vec<String>> {
        self.visible_strings_with_role(role, false).await
    }

    pub(in crate::application::platform::linux) async fn visible_texts_with_role(
        &self,
        role: Role,
    ) -> Result<Vec<String>> {
        self.visible_strings_with_role(role, true).await
    }

    async fn visible_strings_with_role(
        &self,
        role: Role,
        text_contents: bool,
    ) -> Result<Vec<String>> {
        // Observe once. Nodes destroyed during traversal are no longer part of
        // the visible snapshot; other failures still invalidate the observation.
        tokio::time::timeout(super::super::ELEMENT_TIMEOUT, async {
            let mut pending = self.root_children().await?;
            let mut values = Vec::new();
            while let Some(object) = pending.pop() {
                if object.is_null() || !self.includes(&object) { continue; }
                let observed: Result<()> = async {
                        let accessible = object.as_accessible_proxy(self.connection.connection()).await?;
                        if accessible_role_value(&accessible).await? == role as u32
                            && accessible.get_state().await?.contains(State::Showing)
                        {
                            values.push(if text_contents {
                                self.read_text(&Element::new(object.clone())).await?
                            } else {
                                accessible.name().await?
                            });
                        }
                        pending.extend(accessible.get_children().await?);
                    Ok(())
                }.await;
                match observed {
                    Err(error) if is_removed_accessible(&error) => {
                        if let Some(tree) = &self.tree { tree.invalidate().await; }
                        tracing::trace!(%error, "visible accessibility subtree changed during observation");
                    }
                    Err(error) => return Err(error),
                    Ok(()) => {},
                }
            }
            Ok(values)
        }).await.context("timed out observing visible accessibility subtree")?
    }

    pub(in crate::application::platform::linux) async fn ancestor_with_role(
        &self,
        element: &Element,
        role: Role,
    ) -> Result<Element> {
        let mut object = element.object().clone();
        loop {
            let accessible = object
                .as_accessible_proxy(self.connection.connection())
                .await?;
            if accessible_role_value(&accessible).await? == role as u32 {
                return Ok(Element::new(object));
            }
            let parent = accessible.parent().await?;
            if parent.is_null() {
                bail!("element has no {role:?} ancestor");
            }
            object = parent;
        }
    }

    pub(in crate::application::platform::linux) async fn dialog_ancestor(
        &self,
        heading: &Element,
    ) -> Result<Element> {
        let connection = self.connection.connection();
        let mut object = heading.object().clone();
        loop {
            let accessible = tokio::time::timeout(
                ATSPI_RESPONSE_TIMEOUT,
                object.as_accessible_proxy(connection),
            )
            .await
            .context("timed out resolving the dialog heading")??;
            let role = accessible_role_value(&accessible).await?;
            if role == Role::Dialog as u32 || role == Role::Alert as u32 {
                return Ok(Element::new(object));
            }
            let parent = tokio::time::timeout(ATSPI_RESPONSE_TIMEOUT, accessible.parent())
                .await
                .context("timed out resolving the dialog heading parent")??;
            if parent.is_null() {
                bail!("accessible element headed by the expected text has no dialog ancestor");
            }
            object = parent;
        }
    }
}

impl AtspiSession {
    pub(in crate::application::platform::linux) async fn invalidate_tree(&self) {
        if let Some(tree) = &self.tree {
            tree.invalidate().await;
        }
    }
}
