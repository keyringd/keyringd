//! The in-process accessibility driver.
//!
//! Every send is guarded by `respondsToSelector:`, because AppKit's internal
//! accessibility proxies (`NSTableRow`, the cell proxies) implement neither the
//! whole modern `NSAccessibility` protocol nor the whole legacy one. The legacy
//! informal API is what `AXUIElement` bridges to, so reading both is what makes
//! an in-process result match what an external client would see.
//!
//! The driver only searches, operates and reads. It never touches product state
//! or a Reducer, so a scenario cannot pass by driving the model directly.

use std::path::Path;

use keyringd_testkit::macos_ui_protocol::{
    Element, Scope, Selector, Target, UiCommand, UiResponse,
};
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, ProtocolObject};
use objc2::{ClassType, MainThreadMarker, Message, msg_send, sel};
use objc2_app_kit::{
    NSAccessibility, NSApplication, NSBitmapImageFileType, NSColorSpace, NSControl,
    NSControlTextDidChangeNotification, NSDragOperation, NSEvent, NSEventModifierFlags,
    NSEventType, NSMenu, NSPasteboard, NSPasteboardTypeString, NSPasteboardWriting, NSTableView,
    NSTableViewDropOperation, NSTextField, NSUserInterfaceItemIdentification, NSView, NSWindow,
};
use objc2_foundation::{
    NSArray, NSDictionary, NSIndexSet, NSInteger, NSNotification, NSPoint, NSString,
};

use super::test_dragging_info::TestDraggingInfo;

thread_local! {
    static STATUS_MENU: std::cell::RefCell<Option<Retained<NSMenu>>> = const { std::cell::RefCell::new(None) };
}

pub(crate) fn register_status_menu(menu: Retained<NSMenu>) {
    STATUS_MENU.with(|slot| *slot.borrow_mut() = Some(menu));
}

const MAX_DEPTH: usize = 32;

/// The virtual key code AppKit reports for Return.
const RETURN_KEY_CODE: u16 = 36;

pub(super) fn handle(command: UiCommand, mtm: MainThreadMarker) -> anyhow::Result<UiResponse> {
    Ok(match command {
        UiCommand::Tree => UiResponse::Tree { tree: tree(mtm) },
        UiCommand::Elements { scope } => UiResponse::Elements {
            elements: scoped_nodes(&scope, mtm)?
                .iter()
                .map(|object| describe(object))
                .collect(),
        },
        UiCommand::FindElement { target } => UiResponse::Elements {
            elements: locate(&target, mtm)?
                .as_deref()
                .map(describe)
                .into_iter()
                .collect(),
        },
        UiCommand::Press { target } => {
            let element = require(&target, mtm)?;
            let view = resolve_view(&element);
            ensure_interactive(&view)?;
            anyhow::ensure!(
                responds(&view, sel!(accessibilityPerformPress)),
                "element has no press action"
            );
            UiResponse::Pressed {
                accepted: as_accessibility(&view).accessibilityPerformPress(),
            }
        }
        UiCommand::SetText { target, text } => {
            let element = require(&target, mtm)?;
            let view = resolve_view(&element);
            let is_field: bool = unsafe { msg_send![&*view, isKindOfClass: NSTextField::class()] };
            anyhow::ensure!(is_field, "set_text needs a text field");
            ensure_interactive(&view)?;
            let field: &NSTextField =
                unsafe { &*(&*view as *const AnyObject as *const NSTextField) };
            set_text(field, &text, mtm);
            UiResponse::Text {
                value: field.stringValue().to_string(),
            }
        }
        UiCommand::ReturnKey { target } => {
            let element = require(&target, mtm)?;
            let view = resolve_view(&element);
            ensure_interactive(&view)?;
            send_return_key(&view)?;
            UiResponse::Done
        }
        UiCommand::Menu { target, identifier } => {
            let element = require(&target, mtm)?;
            let view = resolve_view(&element);
            ensure_interactive(&view)?;
            choose_menu_item(&view, &identifier, mtm)?;
            UiResponse::Done
        }
        UiCommand::MenuItem { target, identifier } => {
            let element = require(&target, mtm)?;
            let view = resolve_view(&element);
            anyhow::ensure!(responds(&view, sel!(menu)), "element has no menu");
            let menu: Option<Retained<NSMenu>> = unsafe { msg_send![&*view, menu] };
            let menu = menu.ok_or_else(|| anyhow::anyhow!("element has an empty menu"))?;
            let wanted = NSString::from_str(&identifier);
            let items = menu.itemArray();
            let item = items
                .iter()
                .find(|item| item.identifier().as_deref() == Some(&*wanted))
                .ok_or_else(|| anyhow::anyhow!("no menu item with identifier {identifier:?}"))?;
            let mut observed = describe(&item);
            observed.enabled = Some(item.isEnabled());
            observed.hidden = Some(item.isHidden());
            UiResponse::Elements {
                elements: vec![observed],
            }
        }
        UiCommand::ToolbarItem { anchor, identifier } => {
            let windows = scoped_windows(&Scope::Window { anchor }, mtm)?;
            anyhow::ensure!(
                windows.len() == 1,
                "toolbar needs exactly one visible window"
            );
            let toolbar = windows[0]
                .toolbar()
                .ok_or_else(|| anyhow::anyhow!("window has no toolbar"))?;
            let item = toolbar
                .items()
                .iter()
                .find(|item| item.itemIdentifier().to_string() == identifier)
                .ok_or_else(|| anyhow::anyhow!("no toolbar item with identifier {identifier:?}"))?;
            anyhow::ensure!(item.isEnabled(), "toolbar item is disabled");
            let action = item
                .action()
                .ok_or_else(|| anyhow::anyhow!("toolbar item has no action"))?;
            let target = item.target();
            let sent = unsafe {
                NSApplication::sharedApplication(mtm).sendAction_to_from(
                    action,
                    target.as_deref(),
                    Some(&*item),
                )
            };
            anyhow::ensure!(sent, "AppKit did not deliver toolbar action");
            UiResponse::Done
        }
        UiCommand::Identifiers { scope, prefix } => UiResponse::Identifiers {
            identifiers: identifiers(&prefix, &scope, mtm)?,
        },
        UiCommand::DragRow { target, from, to } => {
            let element = require(&target, mtm)?;
            let view = resolve_view(&element);
            ensure_interactive(&view)?;
            drag_row(
                &view,
                NSInteger::try_from(from)?,
                NSInteger::try_from(to)?,
                mtm,
            )?;
            UiResponse::Done
        }
        UiCommand::SelectRow { target, row } => {
            let element = require(&target, mtm)?;
            let view = resolve_view(&element);
            ensure_interactive(&view)?;
            let table = view
                .downcast_ref::<NSTableView>()
                .ok_or_else(|| anyhow::anyhow!("select_row needs a table view"))?;
            let index = NSInteger::try_from(row)?;
            anyhow::ensure!(index < table.numberOfRows(), "table row is out of range");
            if let Some(window) = table.window() {
                window.makeFirstResponder(Some(table));
            }
            table.selectRowIndexes_byExtendingSelection(&NSIndexSet::indexSetWithIndex(row), false);
            table.scrollRowToVisible(index);
            UiResponse::Text {
                value: table.selectedRow().to_string(),
            }
        }
        UiCommand::CloseWindow { target } => {
            let element = require(&target, mtm)?;
            close_window(&resolve_view(&element))?;
            UiResponse::Done
        }
        UiCommand::Pasteboard => {
            let board = crate::system::clipboard::pasteboard(mtm);
            UiResponse::Pasteboard {
                text: unsafe { board.stringForType(NSPasteboardTypeString) }.map(|s| s.to_string()),
            }
        }
        UiCommand::ClearPasteboard => {
            crate::system::clipboard::pasteboard(mtm).clearContents();
            UiResponse::Done
        }
        UiCommand::StatusMenu { identifier } => {
            let menu = STATUS_MENU
                .with(|slot| slot.borrow().clone())
                .ok_or_else(|| anyhow::anyhow!("status menu is not mounted"))?;
            invoke_menu_item(&menu, &identifier, mtm)?;
            UiResponse::Done
        }
        UiCommand::AppMenu {
            submenu,
            identifier,
        } => {
            invoke_app_menu_item(&submenu, &identifier, mtm)?;
            UiResponse::Done
        }
        UiCommand::Screenshot { path, scope } => {
            screenshot(&path, &scope, mtm)?;
            UiResponse::Screenshot { path }
        }
    })
}

fn require(target: &Target, mtm: MainThreadMarker) -> anyhow::Result<Retained<AnyObject>> {
    locate(target, mtm)?
        .ok_or_else(|| anyhow::anyhow!("no accessibility element matching {target:?}"))
}

fn matches_selector(object: &AnyObject, selector: &Selector) -> bool {
    match selector {
        Selector::Id(wanted) => identifier(object).as_ref() == Some(wanted),
        Selector::Value(wanted) => value_of(object).as_ref() == Some(wanted),
        Selector::ValueContaining(wanted) => {
            value_of(object).is_some_and(|value| value.contains(wanted))
        }
    }
}

fn locate(target: &Target, mtm: MainThreadMarker) -> anyhow::Result<Option<Retained<AnyObject>>> {
    let mut candidates = Vec::new();
    let mut seen_views = std::collections::HashSet::new();
    for object in scoped_nodes(&target.scope, mtm)? {
        if matches_selector(&object, &target.selector) {
            let view = resolve_view(&object);
            // A cell and its control view describe the same control.
            if seen_views.insert(&*view as *const AnyObject as usize) {
                candidates.push(object);
            }
        }
    }
    anyhow::ensure!(
        candidates.len() <= 1,
        "ambiguous accessibility query {target:?}: {} candidates",
        candidates.len()
    );
    Ok(candidates.pop())
}

// -- traversal ---------------------------------------------------------------

fn responds(object: &AnyObject, selector: objc2::runtime::Sel) -> bool {
    unsafe { msg_send![object, respondsToSelector: selector] }
}

fn as_accessibility(object: &AnyObject) -> &ProtocolObject<dyn NSAccessibility> {
    // Every AppKit element answers some of the protocol; each send is guarded
    // by `responds` before it is made.
    unsafe { &*(object as *const AnyObject as *const ProtocolObject<dyn NSAccessibility>) }
}

/// The legacy informal accessibility API, which the AppKit proxies answer even
/// when they do not implement the modern protocol.
fn attribute(object: &AnyObject, name: &str) -> Option<Retained<AnyObject>> {
    if !responds(object, sel!(accessibilityAttributeValue:)) {
        return None;
    }
    let name = NSString::from_str(name);
    unsafe { msg_send![object, accessibilityAttributeValue: &*name] }
}

fn string_attribute(object: &AnyObject, name: &str) -> Option<String> {
    let value = attribute(object, name)?;
    if !responds(&value, sel!(isEqualToString:)) {
        return None;
    }
    let value: &NSString = unsafe { &*(&*value as *const AnyObject as *const NSString) };
    Some(value.to_string())
}

/// Sends a Return key press to `view`.
///
/// The element is focused first, because a Return only means anything to the
/// control the user was typing in, and AppKit then routes the event to its
/// field editor: the control's delegate sees the same `insertNewline:` command
/// a typed Return produces. The event is delivered to the window directly, so
/// the fixture does not need to be the active application.
fn send_return_key(view: &AnyObject) -> anyhow::Result<()> {
    anyhow::ensure!(responds(view, sel!(window)), "this element has no window");
    let window: Option<Retained<NSWindow>> = unsafe { msg_send![view, window] };
    let window = window.ok_or_else(|| anyhow::anyhow!("this element is not in a window"))?;
    let characters = NSString::from_str("\r");
    let event = NSEvent::keyEventWithType_location_modifierFlags_timestamp_windowNumber_context_characters_charactersIgnoringModifiers_isARepeat_keyCode(
            NSEventType::KeyDown,
            NSPoint::new(0.0, 0.0),
            NSEventModifierFlags::empty(),
            0.0,
            window.windowNumber(),
            None,
            &characters,
            &characters,
            false,
        RETURN_KEY_CODE,
    )
    .ok_or_else(|| anyhow::anyhow!("AppKit did not build a Return key event"))?;
    let focused: bool = unsafe { msg_send![&*window, makeFirstResponder: view] };
    anyhow::ensure!(focused, "this element did not take keyboard focus");
    window.sendEvent(&event);
    Ok(())
}

/// Closes the window holding `view`, which is what the title bar's close
/// button and ⌘W do. A window offering its own Close button is closed through
/// that button instead, because that is the path the user is given.
fn close_window(view: &AnyObject) -> anyhow::Result<()> {
    anyhow::ensure!(responds(view, sel!(window)), "this element has no window");
    let window: Option<Retained<NSWindow>> = unsafe { msg_send![view, window] };
    let window = window.ok_or_else(|| anyhow::anyhow!("this element is not in a window"))?;
    window.performClose(None);
    Ok(())
}

/// Chooses an item of a pull-down button by identifier.
///
/// Pressing the button through accessibility would open a menu and run
/// AppKit's modal tracking loop on the main thread, which is the same thread
/// that answers this driver, so the run would deadlock. Sending the item's own
/// action instead exercises the product's wiring — the same target and
/// selector the menu would invoke — without displaying the menu. A case that
/// needs the menu to be drawn cannot use this.
fn choose_menu_item(
    view: &AnyObject,
    identifier: &str,
    mtm: MainThreadMarker,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        responds(view, sel!(menu)),
        "the element for {identifier:?} has no menu"
    );
    let menu: Option<Retained<NSMenu>> = unsafe { msg_send![view, menu] };
    let menu =
        menu.ok_or_else(|| anyhow::anyhow!("the element for {identifier:?} has an empty menu"))?;
    invoke_menu_item(&menu, identifier, mtm)
}

fn invoke_menu_item(menu: &NSMenu, identifier: &str, mtm: MainThreadMarker) -> anyhow::Result<()> {
    let wanted = NSString::from_str(identifier);
    let item = menu
        .itemArray()
        .iter()
        .find(|item| item.identifier().as_deref() == Some(&*wanted))
        .ok_or_else(|| {
            let titles = menu
                .itemArray()
                .iter()
                .map(|item| item.title().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            anyhow::anyhow!("no menu item with identifier {identifier:?}; the menu holds {titles}")
        })?;
    anyhow::ensure!(
        item.isEnabled() && !item.isHidden(),
        "menu item is disabled or hidden"
    );
    let Some(action) = item.action() else {
        anyhow::bail!("the menu item {identifier:?} has no action");
    };
    let target = item.target();
    // Delivered through NSApplication rather than `performSelector:`, because
    // an action method returns nothing and the result of `performSelector:`
    // would then be an uninitialised pointer that objc2 tries to retain.
    let sent = unsafe {
        NSApplication::sharedApplication(mtm).sendAction_to_from(
            action,
            target.as_deref(),
            Some(&*item),
        )
    };
    anyhow::ensure!(sent, "AppKit did not deliver the action of {identifier:?}");
    Ok(())
}

/// Invokes a menu item from the application's main menu by submenu and
/// item identifiers. This is how the About panel and similar app-level actions are
/// triggered — there is no view with an identifier to operate on.
fn invoke_app_menu_item(
    submenu_identifier: &str,
    item_identifier: &str,
    mtm: MainThreadMarker,
) -> anyhow::Result<()> {
    let application = NSApplication::sharedApplication(mtm);
    let main_menu = application
        .mainMenu()
        .ok_or_else(|| anyhow::anyhow!("the application has no main menu"))?;
    let wanted_submenu = NSString::from_str(submenu_identifier);
    let submenu = main_menu
        .itemArray()
        .iter()
        .find(|item| item.identifier().as_deref() == Some(&*wanted_submenu))
        .ok_or_else(|| anyhow::anyhow!("no submenu with identifier {submenu_identifier:?}"))?;
    let menu = submenu
        .submenu()
        .ok_or_else(|| anyhow::anyhow!("{submenu_identifier:?} has no submenu"))?;
    let wanted_item = NSString::from_str(item_identifier);
    let item = menu
        .itemArray()
        .iter()
        .find(|item| item.identifier().as_deref() == Some(&*wanted_item))
        .ok_or_else(|| {
            anyhow::anyhow!("no item with identifier {item_identifier:?} in {submenu_identifier:?}")
        })?;
    anyhow::ensure!(
        item.isEnabled() && !item.isHidden(),
        "menu item is disabled or hidden"
    );
    let Some(action) = item.action() else {
        anyhow::bail!("{item_identifier:?} has no action");
    };
    let target = item.target();
    let sent = unsafe { application.sendAction_to_from(action, target.as_deref(), Some(&*item)) };
    anyhow::ensure!(
        sent,
        "AppKit did not deliver the action of {item_identifier:?}"
    );
    Ok(())
}

/// An accessibility child is often a cell, whose identifier lives on the view
/// that owns it.
fn control_view(object: &AnyObject) -> Option<Retained<AnyObject>> {
    if !responds(object, sel!(controlView)) {
        return None;
    }
    unsafe { msg_send![object, controlView] }
}

fn resolve_view(object: &AnyObject) -> Retained<AnyObject> {
    match control_view(object) {
        Some(view) => view,
        None => object.retain(),
    }
}

fn identifier(object: &AnyObject) -> Option<String> {
    let own = if responds(object, sel!(accessibilityIdentifier)) {
        as_accessibility(object)
            .accessibilityIdentifier()
            .map(|value| value.to_string())
            .filter(|value| !value.is_empty())
    } else {
        None
    };
    own.or_else(|| string_attribute(object, "AXIdentifier").filter(|value| !value.is_empty()))
        .or_else(|| identifier(&*control_view(object)?))
}

/// The legacy attribute comes first throughout, because that is the answer an
/// external `AXUIElement` client gets, and matching that client is the point of
/// reading accessibility at all. The modern property is the backstop for an
/// element AppKit does not bridge, such as an `NSAlert` panel, whose `AXTitle`
/// stays the window's own empty title. An empty answer counts as no answer.
fn title(object: &AnyObject) -> Option<String> {
    string_attribute(object, "AXTitle")
        .filter(|title| !title.is_empty())
        .or_else(|| {
            if !responds(object, sel!(accessibilityTitle)) {
                return None;
            }
            as_accessibility(object)
                .accessibilityTitle()
                .map(|title| title.to_string())
                .filter(|title| !title.is_empty())
        })
}

/// Reads an accessibility answer that is a string, or a value that can state
/// itself as one, which is how AppKit returns a switch or a checkbox.
fn describe_string(value: &AnyObject) -> Option<String> {
    if responds(value, sel!(isEqualToString:)) {
        let value: &NSString = unsafe { &*(value as *const AnyObject as *const NSString) };
        return Some(value.to_string());
    }
    if responds(value, sel!(stringValue)) {
        let text: Retained<NSString> = unsafe { msg_send![value, stringValue] };
        return Some(text.to_string());
    }
    None
}

fn role(object: &AnyObject) -> Option<String> {
    if responds(object, sel!(accessibilityRole)) {
        return as_accessibility(object)
            .accessibilityRole()
            .map(|role| role.to_string());
    }
    string_attribute(object, "AXRole")
}

fn children(object: &AnyObject) -> Vec<Retained<AnyObject>> {
    if responds(object, sel!(accessibilityChildren))
        && let Some(list) = as_accessibility(object).accessibilityChildren()
    {
        return list.iter().collect();
    }
    let Some(value) = attribute(object, "AXChildren") else {
        return Vec::new();
    };
    let list: &NSArray = unsafe { &*(&*value as *const AnyObject as *const NSArray) };
    list.iter().collect()
}

fn roots(mtm: MainThreadMarker) -> Vec<Retained<AnyObject>> {
    let application = NSApplication::sharedApplication(mtm);
    application
        .accessibilityWindows()
        .map(|windows| windows.iter().collect())
        .unwrap_or_default()
}

/// Keep each window's search independent: a parent's AXChildren can include
/// its sheet, but a query in the parent must not reach that sheet's controls.
fn window_of(object: &AnyObject) -> Option<Retained<NSWindow>> {
    let is_window: bool = unsafe { msg_send![object, isKindOfClass: NSWindow::class()] };
    if is_window {
        let window: &NSWindow = unsafe { &*(object as *const AnyObject as *const NSWindow) };
        return Some(window.retain());
    }
    let view = resolve_view(object);
    if !responds(&view, sel!(window)) {
        return None;
    }
    unsafe { msg_send![&*view, window] }
}

fn window_nodes(window: &NSWindow) -> anyhow::Result<Vec<Retained<AnyObject>>> {
    let root: &AnyObject = window;
    Ok(keyringd_testkit::accessibility_traversal::collect(
        root.retain(),
        MAX_DEPTH,
        10_000,
        |object| &**object as *const AnyObject as usize,
        |object| window_of(object).is_none_or(|owner| std::ptr::eq(&*owner, window)),
        |object| children(object),
    )?)
}

fn visible_windows(mtm: MainThreadMarker) -> Vec<Retained<NSWindow>> {
    // NSApplication.windows also includes attached sheets. Deduplicate because
    // AppKit may expose them through both the parent and application.
    let mut windows: Vec<_> = NSApplication::sharedApplication(mtm)
        .windows()
        .iter()
        .filter(|w| w.isVisible())
        .collect();
    for window in windows.clone() {
        if let Some(sheet) = window.attachedSheet()
            && sheet.isVisible()
            && !windows.iter().any(|w| std::ptr::eq(&**w, &*sheet))
        {
            windows.push(sheet);
        }
    }
    windows
}

fn scoped_windows(scope: &Scope, mtm: MainThreadMarker) -> anyhow::Result<Vec<Retained<NSWindow>>> {
    let application = NSApplication::sharedApplication(mtm);
    match scope {
        Scope::Application => Ok(visible_windows(mtm)),
        Scope::KeyWindow => Ok(application
            .keyWindow()
            .filter(|w| w.isVisible())
            .into_iter()
            .collect()),
        Scope::WindowContaining { selector } => {
            let mut owners = Vec::new();
            for window in visible_windows(mtm) {
                if window_nodes(&window)?
                    .iter()
                    .any(|object| matches_selector(object, selector))
                {
                    owners.push(window);
                }
            }
            anyhow::ensure!(
                owners.len() <= 1,
                "window selector {selector:?} matches {} windows",
                owners.len()
            );
            Ok(owners)
        }
        Scope::Window { anchor } | Scope::Sheet { anchor } => {
            let mut owners = Vec::new();
            for window in visible_windows(mtm) {
                if window_nodes(&window)?
                    .iter()
                    .any(|object| identifier(object).as_ref() == Some(anchor))
                {
                    owners.push(window);
                }
            }
            anyhow::ensure!(
                owners.len() <= 1,
                "window anchor {anchor:?} matches {} windows",
                owners.len()
            );
            let Some(owner) = owners.pop() else {
                return Ok(Vec::new());
            };
            if matches!(scope, Scope::Sheet { .. }) {
                Ok(owner
                    .attachedSheet()
                    .filter(|w| w.isVisible())
                    .into_iter()
                    .collect())
            } else {
                Ok(vec![owner])
            }
        }
    }
}

fn scoped_nodes(scope: &Scope, mtm: MainThreadMarker) -> anyhow::Result<Vec<Retained<AnyObject>>> {
    let mut nodes = Vec::new();
    for window in scoped_windows(scope, mtm)? {
        nodes.extend(window_nodes(&window)?);
    }
    Ok(nodes)
}

fn identifiers(prefix: &str, scope: &Scope, mtm: MainThreadMarker) -> anyhow::Result<Vec<String>> {
    let mut out = Vec::new();
    for object in scoped_nodes(scope, mtm)? {
        if let Some(id) = identifier(&object)
            && id.starts_with(prefix)
            && !out.contains(&id)
        {
            out.push(id);
        }
    }
    Ok(out)
}

/// Moves a table row by handing the product's own drop handler the pasteboard
/// its own drag source produced. See [`super::test_dragging_info`] for why the drag
/// itself cannot be performed.
fn drag_row(
    view: &AnyObject,
    from: NSInteger,
    to: NSInteger,
    mtm: MainThreadMarker,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        responds(view, sel!(dataSource)),
        "this element is not a table"
    );
    let table: &NSTableView = unsafe { &*(view as *const AnyObject as *const NSTableView) };
    let data_source = unsafe { table.dataSource() }
        .ok_or_else(|| anyhow::anyhow!("this table has no data source"))?;
    let writer: Option<Retained<AnyObject>> =
        unsafe { msg_send![&*data_source, tableView: table, pasteboardWriterForRow: from] };
    let writer = writer
        .ok_or_else(|| anyhow::anyhow!("the product offers nothing to drag from row {from}"))?;
    let pasteboard = NSPasteboard::pasteboardWithUniqueName();
    pasteboard.clearContents();
    let writer: &ProtocolObject<dyn NSPasteboardWriting> = unsafe {
        &*(&*writer as *const AnyObject as *const ProtocolObject<dyn NSPasteboardWriting>)
    };
    pasteboard.writeObjects(&NSArray::from_slice(&[writer]));
    let info = TestDraggingInfo::new(mtm, pasteboard.clone());
    let allowed: NSDragOperation = unsafe {
        msg_send![
            &*data_source,
            tableView: table,
            validateDrop: &*info,
            proposedRow: to,
            proposedDropOperation: NSTableViewDropOperation::Above,
        ]
    };
    let accepted: bool = if allowed == NSDragOperation::None {
        false
    } else {
        unsafe {
            msg_send![
                &*data_source,
                tableView: table,
                acceptDrop: &*info,
                row: to,
                dropOperation: NSTableViewDropOperation::Above,
            ]
        }
    };
    // The pasteboard is private to this drop and never the user's clipboard.
    let _: () = unsafe { msg_send![&*pasteboard, releaseGlobally] };
    anyhow::ensure!(
        allowed != NSDragOperation::None,
        "the product refused a drop of row {from} above row {to}"
    );
    anyhow::ensure!(
        accepted,
        "the product did not accept the drop of row {from} above row {to}"
    );
    Ok(())
}

// -- reading -----------------------------------------------------------------

fn describe(element: &AnyObject) -> Element {
    let view = resolve_view(element);
    Element {
        role: role(element),
        identifier: identifier(element),
        title: title(element),
        value: value_of(element),
        enabled: enabled(&view),
        hidden: hidden(&view),
        class: view.class().name().to_string_lossy().into_owned(),
    }
}

/// Legacy first, for the same reason as [`title`]. Each stage falls through to
/// the next when it has no answer, so the modern property is reached only by an
/// element AppKit bridges to none of them.
fn value_of(element: &AnyObject) -> Option<String> {
    if let Some(value) = string_attribute(element, "AXValue") {
        return Some(value);
    }
    if let Some(value) = attribute(element, "AXValue")
        && responds(&value, sel!(stringValue))
    {
        let text: Retained<NSString> = unsafe { msg_send![&*value, stringValue] };
        return Some(text.to_string());
    }
    let view = resolve_view(element);
    if responds(&view, sel!(stringValue)) {
        let value: Option<Retained<NSString>> = unsafe { msg_send![&*view, stringValue] };
        if let Some(value) = value {
            return Some(value.to_string());
        }
    }
    if !responds(element, sel!(accessibilityValue)) {
        return None;
    }
    let value = as_accessibility(element).accessibilityValue()?;
    describe_string(&value)
}

/// `AXEnabled` is absent on plain views, so the control's own state is the
/// fallback, and "no answer either way" stays `None` instead of a guess.
fn enabled(view: &AnyObject) -> Option<bool> {
    if let Some(value) = attribute(view, "AXEnabled")
        && responds(&value, sel!(boolValue))
    {
        let value: bool = unsafe { msg_send![&*value, boolValue] };
        return Some(value);
    }
    if responds(view, sel!(isEnabled)) {
        let control: &NSControl = unsafe { &*(view as *const AnyObject as *const NSControl) };
        return Some(control.isEnabled());
    }
    None
}

fn hidden(view: &AnyObject) -> Option<bool> {
    if !responds(view, sel!(isHiddenOrHasHiddenAncestor)) {
        return None;
    }
    let view: &NSView = unsafe { &*(view as *const AnyObject as *const NSView) };
    Some(view.isHiddenOrHasHiddenAncestor())
}

fn tree(mtm: MainThreadMarker) -> String {
    fn walk(object: &AnyObject, depth: usize, budget: &mut usize, out: &mut String) {
        if *budget == 0 || depth > MAX_DEPTH {
            return;
        }
        *budget -= 1;
        out.push_str(&"  ".repeat(depth));
        out.push_str(role(object).as_deref().unwrap_or("<no role>"));
        out.push_str(&format!(" [{}]", object.class().name().to_string_lossy()));
        if let Some(identifier) = identifier(object) {
            out.push_str(&format!(" id={identifier:?}"));
        }
        if let Some(title) = title(object) {
            out.push_str(&format!(" title={title:?}"));
        }
        if let Some(value) = value_of(object).filter(|value| !value.is_empty()) {
            out.push_str(&format!(" value={value:?}"));
        }
        out.push('\n');
        for child in children(object) {
            walk(&child, depth + 1, budget, out);
        }
    }
    let mut out = String::new();
    let mut budget = 600usize;
    for root in roots(mtm) {
        walk(&root, 0, &mut budget, &mut out);
    }
    out
}

// -- operating ---------------------------------------------------------------

/// Typing through accessibility is not available for `NSTextField`, so the edit
/// is made the way AppKit makes it: focus the field, change its value, then
/// deliver the same delegate callback a keystroke would.
fn set_text(field: &NSTextField, text: &str, _mtm: MainThreadMarker) {
    let text = NSString::from_str(text);
    if let Some(window) = field.window() {
        window.makeFirstResponder(Some(field));
    }
    field.setStringValue(&text);
    let delegate: Option<Retained<AnyObject>> = unsafe { msg_send![field, delegate] };
    let Some(delegate) = delegate else {
        return;
    };
    if !responds(&delegate, sel!(controlTextDidChange:)) {
        return;
    }
    let notification = unsafe {
        NSNotification::notificationWithName_object(NSControlTextDidChangeNotification, Some(field))
    };
    let _: () = unsafe { msg_send![&*delegate, controlTextDidChange: &*notification] };
}

// -- evidence ----------------------------------------------------------------

/// Draws the full window frame, including its toolbar, into an opaque image.
/// Nothing outside the
/// process is read, so no Screen Recording permission is involved.
///
/// The window is chosen by an element it contains, because a sheet is its own
/// `NSWindow` with no title and picking by title would silently capture the
/// parent instead.
fn screenshot(path: &Path, scope: &Scope, mtm: MainThreadMarker) -> anyhow::Result<()> {
    let mut windows = scoped_windows(scope, mtm)?;
    anyhow::ensure!(
        windows.len() == 1,
        "screenshot requires exactly one window, found {} for {scope:?}",
        windows.len()
    );
    let window = windows.pop().unwrap();
    let content = window
        .contentView()
        .ok_or_else(|| anyhow::anyhow!("window has no content view"))?;
    let view = unsafe { content.superview() }
        .ok_or_else(|| anyhow::anyhow!("window has no frame view"))?;
    view.layoutSubtreeIfNeeded();
    window.displayIfNeeded();
    let bounds = view.bounds();
    let representation = view
        .bitmapImageRepForCachingDisplayInRect(bounds)
        .ok_or_else(|| anyhow::anyhow!("window is not backed by a bitmap"))?;
    view.cacheDisplayInRect_toBitmapImageRep(bounds, &representation);
    let png = unsafe {
        representation
            .representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
    }
    .ok_or_else(|| anyhow::anyhow!("could not encode the window as PNG"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // AppKit leaves non-opaque NSViews transparent. Resolve that transparency
    // against this window's background so the report's theme cannot hide text.
    let background = window
        .backgroundColor()
        .colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())
        .ok_or_else(|| anyhow::anyhow!("window background cannot be represented in sRGB"))?;
    let background = [
        background.redComponent(),
        background.greenComponent(),
        background.blueComponent(),
    ]
    .map(|component| (component.clamp(0.0, 1.0) * 255.0).round() as u32);
    let mut pixels = image::load_from_memory(&png.to_vec())?.to_rgba8();
    for pixel in pixels.pixels_mut() {
        let alpha = u32::from(pixel[3]);
        for channel in 0..3 {
            pixel[channel] =
                ((u32::from(pixel[channel]) * alpha + background[channel] * (255 - alpha) + 127)
                    / 255) as u8;
        }
        pixel[3] = 255;
    }
    image::DynamicImage::ImageRgba8(pixels).write_to(
        &mut std::io::BufWriter::new(std::fs::File::create(path)?),
        image::ImageFormat::Png,
    )?;
    Ok(())
}

fn ensure_interactive(view: &AnyObject) -> anyhow::Result<()> {
    anyhow::ensure!(hidden(view) != Some(true), "element is hidden");
    anyhow::ensure!(enabled(view) != Some(false), "element is disabled");
    Ok(())
}
