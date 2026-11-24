use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, ensure};

use crate::environment::macos::TestApp;

const SETTLE: Duration = Duration::from_millis(25);
const TIMEOUT: Duration = Duration::from_secs(10);

pub use keyringd_testkit::macos_ui_protocol::Element;
use keyringd_testkit::macos_ui_protocol::{
    ControlCommand, ControlResponse, FixtureState, Scope, Selector, Target, UiCommand, UiResponse,
};

/// Operates the product UI by accessibility identifier.
///
/// The UI runs on the fixture's main thread while `NSApplication` is running,
/// so every call here is a round trip that the fixture hops onto that thread.
pub struct UiDriver<'a> {
    app: &'a TestApp,
    scope: Scope,
}

impl<'a> UiDriver<'a> {
    pub fn new(app: &'a TestApp) -> Self {
        Self {
            app,
            scope: Scope::Application,
        }
    }

    pub fn in_window(&self, anchor: &str) -> Self {
        Self {
            app: self.app,
            scope: Scope::Window {
                anchor: anchor.into(),
            },
        }
    }

    pub fn in_sheet(&self, anchor: &str) -> Self {
        Self {
            app: self.app,
            scope: Scope::Sheet {
                anchor: anchor.into(),
            },
        }
    }

    fn target(&self, selector: Selector) -> Target {
        Target {
            scope: self.scope.clone(),
            selector,
        }
    }

    fn id(&self, id: &str) -> Target {
        self.target(Selector::Id(id.into()))
    }

    fn send(&self, operation: UiCommand) -> Result<UiResponse> {
        match self.app.control(ControlCommand {
            ui: Some(operation),
            ..Default::default()
        })? {
            ControlResponse::Ui { result } => Ok(result),
            other => anyhow::bail!("expected UI response, got {other:?}"),
        }
    }

    fn done(&self, command: UiCommand) -> Result<()> {
        match self.send(command)? {
            UiResponse::Done => Ok(()),
            other => anyhow::bail!("expected action acknowledgement, got {other:?}"),
        }
    }

    pub(crate) fn wait_for_fixture_state(
        &self,
        description: &str,
        predicate: impl Fn(&FixtureState) -> bool,
    ) -> Result<FixtureState> {
        self.app.wait_control(description, predicate)
    }

    pub fn elements(&self) -> Result<Vec<Element>> {
        match self.send(UiCommand::Elements {
            scope: self.scope.clone(),
        })? {
            UiResponse::Elements { elements } => Ok(elements),
            other => anyhow::bail!("expected accessibility elements, got {other:?}"),
        }
    }

    pub fn element(&self, id: &str) -> Result<Element> {
        self.find_element(id)?
            .with_context(|| format!("no accessibility element with id {id:?}"))
    }

    pub fn find_element(&self, id: &str) -> Result<Option<Element>> {
        self.lookup(Selector::Id(id.into()))
    }
    // Value-based locators are only for system-owned panels, whose text AppKit
    // draws itself: the standard About panel and `NSAlert` sheets.
    pub fn find_valued(&self, value: &str) -> Result<Option<Element>> {
        self.lookup(Selector::Value(value.into()))
    }

    fn lookup(&self, selector: Selector) -> Result<Option<Element>> {
        match self.send(UiCommand::FindElement {
            target: self.target(selector),
        })? {
            UiResponse::Elements { mut elements } => {
                ensure!(
                    elements.len() <= 1,
                    "ambiguous accessibility query in {:?}: {elements:?}",
                    self.scope
                );
                Ok(elements.pop())
            }
            other => anyhow::bail!("expected element lookup, got {other:?}"),
        }
    }

    /// An element only exists once its window is on screen, so appearing is
    /// itself the thing a case waits for.
    pub fn wait_for(&self, id: &str) -> Result<Element> {
        self.wait_until(id, "to appear", |element| element.hidden != Some(true))
    }

    pub fn wait_until(
        &self,
        id: &str,
        expectation: &str,
        predicate: impl Fn(&Element) -> bool,
    ) -> Result<Element> {
        self.wait_for_query(Selector::Id(id.into()), expectation, |element| {
            element.hidden != Some(true) && predicate(element)
        })
    }

    fn wait_for_query(
        &self,
        selector: Selector,
        expectation: &str,
        predicate: impl Fn(&Element) -> bool,
    ) -> Result<Element> {
        self.wait_for_observation(&format!("{selector:?} {expectation}"), || {
            let element = self.lookup(selector.clone())?;
            let observed = format!("{element:?}");
            Ok((element.filter(|element| predicate(element)), observed))
        })
    }

    pub(crate) fn wait_for_observation<T>(
        &self,
        expectation: &str,
        mut observe: impl FnMut() -> Result<(Option<T>, String)>,
    ) -> Result<T> {
        self.wait_with_timeout(expectation, TIMEOUT, &mut observe)
    }

    pub(crate) fn wait_with_timeout<T>(
        &self,
        expectation: &str,
        timeout: Duration,
        observe: &mut impl FnMut() -> Result<(Option<T>, String)>,
    ) -> Result<T> {
        let deadline = Instant::now() + timeout;
        loop {
            let (result, last) = observe()
                .with_context(|| format!("while waiting for {expectation} in {:?}", self.scope))?;
            if let Some(result) = result {
                return Ok(result);
            }
            if Instant::now() >= deadline {
                let tree = self
                    .tree()
                    .unwrap_or_else(|error| format!("AX tree unavailable: {error:#}"));
                anyhow::bail!(
                    "timed out waiting for {expectation} in {:?}; last observation: {last}\n{tree}",
                    self.scope
                );
            }
            std::thread::sleep(SETTLE.min(deadline.saturating_duration_since(Instant::now())));
        }
    }

    /// Transport failures never count as evidence that a window closed.
    pub fn wait_absent(&self, id: &str) -> Result<()> {
        self.wait_for_observation(&format!("{id:?} to disappear"), || {
            let element = self.find_element(id)?;
            Ok((element.is_none().then_some(()), format!("{element:?}")))
        })
    }

    pub fn wait_valued(&self, value: &str) -> Result<()> {
        self.wait_for_query(Selector::Value(value.into()), "to appear", |element| {
            element.hidden != Some(true)
        })?;
        Ok(())
    }

    pub fn in_window_with_value_containing(&self, value: &str) -> Self {
        Self {
            app: self.app,
            scope: Scope::WindowContaining {
                selector: Selector::ValueContaining(value.into()),
            },
        }
    }

    pub fn close_valued_window(&self, value: &str) -> Result<()> {
        self.done(UiCommand::CloseWindow {
            target: self.target(Selector::Value(value.into())),
        })
    }

    pub fn wait_value_absent(&self, value: &str) -> Result<()> {
        self.wait_for_observation(&format!("value {value:?} to disappear"), || {
            let element = self.find_valued(value)?;
            Ok((element.is_none().then_some(()), format!("{element:?}")))
        })
    }

    pub fn wait_value(&self, id: &str, expected: &str) -> Result<()> {
        self.wait_until(id, &format!("to read {expected:?}"), |element| {
            element.hidden != Some(true) && element.value.as_deref() == Some(expected)
        })?;
        Ok(())
    }

    /// `accessibilityPerformPress` reports success unreliably on AppKit
    /// controls, so the return value is deliberately discarded: a case must
    /// confirm a press by its effect on the UI or on the product state.
    pub fn press(&self, id: &str) -> Result<()> {
        self.press_target(self.id(id))
    }

    pub fn select_row(&self, id: &str, row: usize) -> Result<()> {
        match self.send(UiCommand::SelectRow {
            target: self.id(id),
            row,
        })? {
            UiResponse::Text { value } => {
                ensure!(
                    value == row.to_string(),
                    "table did not select row {row}: {value}"
                );
                Ok(())
            }
            other => anyhow::bail!("expected selected row, got {other:?}"),
        }
    }

    fn press_target(&self, target: Target) -> Result<()> {
        match self.send(UiCommand::Press { target })? {
            UiResponse::Pressed { .. } => Ok(()),
            other => anyhow::bail!("expected press response, got {other:?}"),
        }
    }

    /// Sends Return to the window holding the element, which is how a field
    /// being edited is committed.
    pub fn press_return(&self, id: &str) -> Result<()> {
        self.done(UiCommand::ReturnKey {
            target: self.id(id),
        })
    }

    /// Observes a native menu item without starting AppKit modal menu tracking.
    pub fn menu_item(&self, id: &str, identifier: &str) -> Result<Element> {
        match self.send(UiCommand::MenuItem {
            target: self.id(id),
            identifier: identifier.into(),
        })? {
            UiResponse::Elements { mut elements } => {
                ensure!(
                    elements.len() == 1,
                    "expected one menu item, got {elements:?}"
                );
                Ok(elements.remove(0))
            }
            other => anyhow::bail!("expected menu element response, got {other:?}"),
        }
    }

    /// Chooses an item of a pull-down button by its identifier. The menu is not
    /// displayed: opening it would run AppKit's modal tracking loop on the
    /// thread that answers the driver. The item's own action is sent instead.
    pub fn choose_menu_item(&self, id: &str, identifier: &str) -> Result<()> {
        self.done(UiCommand::Menu {
            target: self.id(id),
            identifier: identifier.into(),
        })
    }

    /// Invokes the native toolbar item by its stable itemIdentifier.
    pub fn invoke_toolbar_item(&self, anchor: &str, identifier: &str) -> Result<()> {
        self.done(UiCommand::ToolbarItem {
            anchor: anchor.into(),
            identifier: identifier.into(),
        })
    }

    /// Closes the window holding the element, the way its close button does.
    pub fn close_window(&self, id: &str) -> Result<()> {
        self.done(UiCommand::CloseWindow {
            target: self.id(id),
        })
    }

    /// Clears the fixture's private pasteboard before checking a copy action.
    pub fn clear_pasteboard(&self) -> Result<()> {
        self.done(UiCommand::ClearPasteboard)
    }

    /// Waits for text to reach the fixture pasteboard, because copying is an
    /// effect the product runs after the press returns.
    pub fn wait_pasteboard_text(&self) -> Result<String> {
        self.wait_for_observation("copied text to reach the pasteboard", || {
            let UiResponse::Pasteboard { text } = self.send(UiCommand::Pasteboard)? else {
                anyhow::bail!("expected pasteboard response");
            };
            let observed = format!("{text:?}");
            Ok((text.filter(|text| !text.is_empty()), observed))
        })
    }

    /// The identifiers beginning with `prefix`, in the order they are laid out,
    /// which is how a case reads the order of a list.
    pub fn identifiers(&self, prefix: &str) -> Result<Vec<String>> {
        match self.send(UiCommand::Identifiers {
            scope: self.scope.clone(),
            prefix: prefix.into(),
        })? {
            UiResponse::Identifiers { identifiers } => Ok(identifiers),
            other => anyhow::bail!("expected identifiers response, got {other:?}"),
        }
    }

    /// Moves a row of the list `id` above the row now at `to`. The product's
    /// own drop handling runs; the drag itself cannot be performed in process.
    pub fn drag_row(&self, id: &str, from: usize, to: usize) -> Result<()> {
        self.done(UiCommand::DragRow {
            target: self.id(id),
            from,
            to,
        })
    }

    /// Returns what the field holds afterwards, which is what the product's
    /// own delegate saw, not what was requested.
    pub fn enter_text(&self, id: &str, text: &str) -> Result<String> {
        match self.send(UiCommand::SetText {
            target: self.id(id),
            text: text.into(),
        })? {
            UiResponse::Text { value } => Ok(value),
            other => anyhow::bail!("expected text response, got {other:?}"),
        }
    }

    /// Captures the window holding `containing`, drawn from the fixture's own
    /// backing store. A sheet is its own untitled window, so it is identified
    /// by an element inside it rather than by a title.
    pub fn capture(&self, containing: &str) -> Result<PathBuf> {
        let directory = PathBuf::from(
            std::env::var_os("KEYRINGD_TEST_REPORT_DIR")
                .context("report directory is required for screenshots")?,
        )
        .join("screenshots");
        std::fs::create_dir_all(&directory)?;
        let name = format!("{}.png", unique_name());
        let path = directory.join(&name);
        self.capture_to(
            &path,
            Scope::Window {
                anchor: containing.into(),
            },
        )?;
        ensure!(
            path.exists(),
            "the fixture reported a screenshot it did not write"
        );
        // The report links evidence relative to its own directory, so that the
        // whole directory stays valid when it is moved or uploaded.
        Ok(Path::new("screenshots").join(name))
    }

    pub fn capture_current_window(&self) -> Result<PathBuf> {
        let directory = PathBuf::from(
            std::env::var_os("KEYRINGD_TEST_REPORT_DIR")
                .context("report directory is required for screenshots")?,
        )
        .join("screenshots");
        std::fs::create_dir_all(&directory)?;
        let name = format!("{}.png", unique_name());
        let path = directory.join(&name);
        self.capture_to(&path, self.scope.clone())?;
        ensure!(path.is_file(), "fixture did not write screenshot");
        Ok(Path::new("screenshots").join(name))
    }

    /// Decodes the pixels rendered by the product, rather than reading the
    /// FIDO client's internal QR payload. Returns the image as report evidence.
    pub fn read_hybrid_qr(&self, containing: &str) -> Result<Option<(String, PathBuf)>> {
        let screenshot = self.capture(containing)?;
        let report = PathBuf::from(
            std::env::var_os("KEYRINGD_TEST_REPORT_DIR")
                .context("report directory is required for QR decoding")?,
        );
        Ok(
            keyringd_testkit::QrCodeReader::read_hybrid(&report.join(&screenshot))?
                .map(|qr| (qr, screenshot)),
        )
    }

    /// Invokes a menu item from the application's main menu bar by submenu
    /// and item identifiers.
    pub fn invoke_app_menu_item(&self, submenu: &str, identifier: &str) -> Result<()> {
        self.done(UiCommand::AppMenu {
            submenu: submenu.into(),
            identifier: identifier.into(),
        })
    }

    /// Sends the identified status menu item's real target/action. Menu drawing and mouse
    /// tracking are outside this in-process driver's scope.
    pub fn invoke_status_menu_item(&self, identifier: &str) -> Result<()> {
        self.done(UiCommand::StatusMenu {
            identifier: identifier.into(),
        })
    }

    /// The whole accessibility tree, for diagnosing a failing locator.
    pub fn tree(&self) -> Result<String> {
        match self.send(UiCommand::Tree)? {
            UiResponse::Tree { tree } => Ok(tree),
            other => anyhow::bail!("expected tree response, got {other:?}"),
        }
    }

    pub(crate) fn capture_to(&self, path: &Path, scope: Scope) -> Result<()> {
        match self.send(UiCommand::Screenshot {
            path: path.into(),
            scope,
        })? {
            UiResponse::Screenshot { path: written } if written == path => Ok(()),
            other => anyhow::bail!("unexpected screenshot response: {other:?}"),
        }
    }

    pub fn wait_hybrid_qr(&self, containing: &str) -> Result<(String, PathBuf)> {
        self.wait_fresh_hybrid_qr(containing, None)
    }

    pub fn wait_fresh_hybrid_qr(
        &self,
        containing: &str,
        previous: Option<&str>,
    ) -> Result<(String, PathBuf)> {
        self.wait_for_observation(&format!("a fresh Hybrid QR in {containing:?}"), || {
            let qr = self.read_hybrid_qr(containing)?;
            let observed = match &qr {
                None => "no decodable Hybrid QR".to_owned(),
                Some((uri, image)) => format!(
                    "QR {} the previous session; image: {}",
                    if previous == Some(uri.as_str()) {
                        "matches"
                    } else {
                        "differs from"
                    },
                    image.display()
                ),
            };
            Ok((
                qr.filter(|(uri, _)| previous != Some(uri.as_str())),
                observed,
            ))
        })
    }
}

/// Distinct within a run without pulling in a UUID dependency for a filename.
fn unique_name() -> String {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(
        "{}-{}-{}",
        std::process::id(),
        nanos,
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
}
