use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::application::platform::windows::ComApartment;
use anyhow::{Context, Result, bail, ensure};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::*;
use windows::core::{BSTR, Interface};

const TIMEOUT: Duration = Duration::from_secs(15);

/// A case's dedicated test thread owns all COM interfaces; this driver cannot cross threads.
pub struct UiDriver {
    automation: IUIAutomation,
    _apartment: ComApartment,
}

impl UiDriver {
    pub(crate) fn diagnose_windows(&self, pid: u32) -> String {
        let mut diagnostics = Vec::new();
        match crate::application::platform::windows::window_locator::visible_windows(pid) {
            Ok(windows) => {
                for handle in windows {
                    diagnostics.push(self.window_diagnostics(handle.0 as usize));
                    match self.element_from_handle(handle.0 as usize) {
                        Ok(root) => {
                            diagnostics.push(format!("UIA root: {:?}", unsafe {
                                (
                                    root.CurrentName(),
                                    root.CurrentAutomationId(),
                                    root.CurrentIsOffscreen(),
                                )
                            }));
                            diagnostics
                                .push(format!("UIA controls: {:?}", self.control_inventory(&root)));
                            match self.capture(&root) {
                                Ok(image) => {
                                    let _ = keyringd_testkit::record_image(
                                        "Failure window diagnostics",
                                        &image,
                                    );
                                    diagnostics.push(format!("Screenshot: {}", image.display()));
                                }
                                Err(error) => {
                                    diagnostics.push(format!("Screenshot failed: {error:#}"))
                                }
                            }
                        }
                        Err(error) => diagnostics.push(format!("UIA root failed: {error:#}")),
                    }
                }
            }
            Err(error) => diagnostics.push(format!("Window enumeration failed: {error:#}")),
        }
        diagnostics.join("\n")
    }

    pub fn close_window(&self, window: &IUIAutomationElement) -> Result<()> {
        unsafe {
            let pattern: IUIAutomationWindowPattern =
                window.GetCurrentPatternAs(UIA_WindowPatternId)?;
            pattern.Close()?;
        }
        Ok(())
    }

    pub fn contains_automation_id(&self, root: &IUIAutomationElement, id: &str) -> Result<bool> {
        unsafe {
            let condition = self
                .automation
                .CreatePropertyCondition(UIA_AutomationIdPropertyId, &VARIANT::from(id))?;
            Ok(root.FindAll(TreeScope_Descendants, &condition)?.Length()? != 0)
        }
    }
    pub fn open_tray_menu(&self, pid: u32) -> Result<()> {
        use windows::Win32::Foundation::{LPARAM, WPARAM};
        use windows::Win32::UI::Shell::{NOTIFYICONIDENTIFIER, Shell_NotifyIconGetRect};
        use windows::Win32::UI::WindowsAndMessaging::{
            GUITHREADINFO, GetClassNameW, GetGUIThreadInfo, GetWindowThreadProcessId, PostMessageW,
            WM_RBUTTONUP,
        };
        use windows::core::GUID;
        let guid =
            std::env::var("KEYRINGD_UI_TEST_TRAY_GUID").context("missing fixture tray GUID")?;
        let icon = NOTIFYICONIDENTIFIER {
            cbSize: size_of::<NOTIFYICONIDENTIFIER>() as u32,
            guidItem: GUID::try_from(guid.as_str())?,
            ..Default::default()
        };
        // tray-icon ignores clicks until Explorer can resolve the icon's rectangle.
        self.wait("fixture tray icon in Explorer", || {
            Ok(unsafe { Shell_NotifyIconGetRect(&icon) }.ok())
        })?;
        let window = self.wait("fixture Tray window", || {
            Ok(
                crate::application::platform::windows::window_locator::windows(pid, false)?
                    .into_iter()
                    .find(|window| {
                        let mut class = [0u16; 128];
                        let length = unsafe { GetClassNameW(*window, &mut class) };
                        String::from_utf16_lossy(&class[..length as usize]) == "tray_icon_app"
                    }),
            )
        })?;
        // tray-icon receives this notification from Explorer. Deliver the
        // right-click to our own icon; do not depend on Explorer overflow layout.
        unsafe {
            PostMessageW(Some(window), 6002, WPARAM(0), LPARAM(WM_RBUTTONUP as isize))?;
        }
        let thread = unsafe { GetWindowThreadProcessId(window, None) };
        self.wait("fixture tray popup menu", || {
            let mut info = GUITHREADINFO {
                cbSize: size_of::<GUITHREADINFO>() as u32,
                ..Default::default()
            };
            unsafe { GetGUIThreadInfo(thread, &mut info) }?;
            Ok((info.hwndMenuOwner == window).then_some(()))
        })
        .with_context(|| {
            format!(
                "Tray notification sent to {}; foreground: {}; fixture windows: {:?}",
                self.window_diagnostics(window.0 as usize),
                self.window_diagnostics(
                    unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() }.0
                        as usize
                ),
                crate::application::platform::windows::window_locator::windows(pid, false).map(
                    |windows| {
                        windows
                            .into_iter()
                            .map(|window| self.window_diagnostics(window.0 as usize))
                            .collect::<Vec<_>>()
                    }
                )
            )
        })
    }

    pub fn press_named_enter(&self, root: &IUIAutomationElement, name: &str) -> Result<()> {
        self.press_element_enter(root, &self.element_named(root, name)?)
    }
    pub fn new() -> Result<Self> {
        let apartment = ComApartment::new()?;
        let automation = unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)? };
        Ok(Self {
            automation,
            _apartment: apartment,
        })
    }

    pub fn control_inventory(&self, root: &IUIAutomationElement) -> Result<Vec<String>> {
        unsafe {
            let condition = self.automation.CreateTrueCondition()?;
            let elements = root.FindAll(TreeScope_Descendants, &condition)?;
            let mut result = Vec::new();
            for index in 0..elements.Length()?.min(120) {
                let element = elements.GetElement(index)?;
                result.push(format!(
                    "{} | {} | {}",
                    element.CurrentControlType()?.0,
                    element.CurrentAutomationId()?,
                    element.CurrentName()?,
                ));
            }
            Ok(result)
        }
    }

    pub fn visible_radio_labels(&self, root: &IUIAutomationElement) -> Result<Vec<String>> {
        unsafe {
            let radios = self
                .automation
                .CreatePropertyCondition(
                    UIA_ControlTypePropertyId,
                    &VARIANT::from(UIA_RadioButtonControlTypeId.0),
                )
                .context("create visible radio labels ControlType condition")?;
            let visible = self
                .automation
                .CreatePropertyCondition(UIA_IsOffscreenPropertyId, &VARIANT::from(false))
                .context("create visible radio labels IsOffscreen condition")?;
            let condition = self
                .automation
                .CreateAndCondition(&radios, &visible)
                .context("combine visible radio labels conditions")?;
            let elements = root
                .FindAll(TreeScope_Descendants, &condition)
                .context("find visible radio label descendants")?;
            let length = elements
                .Length()
                .context("read visible radio labels count")?;
            (0..length)
                .map(|index| {
                    let element = elements.GetElement(index).with_context(|| {
                        format!("read visible radio label element {index}/{length}")
                    })?;
                    Ok(element
                        .CurrentName()
                        .with_context(|| format!("read visible radio label Name {index}/{length}"))?
                        .to_string())
                })
                .collect()
        }
    }

    pub fn visible_window_descriptions(&self) -> Result<Vec<String>> {
        crate::application::platform::windows::window_locator::visible_window_descriptions()
    }

    pub fn windows_security_handles(&self) -> Result<Vec<usize>> {
        Ok(
            crate::application::platform::windows::window_locator::visible_windows_named(
                "Windows Security",
            )?
            .into_iter()
            .map(|window| window.0 as usize)
            .collect(),
        )
    }

    pub fn element_from_handle(&self, handle: usize) -> Result<IUIAutomationElement> {
        Ok(unsafe {
            self.automation
                .ElementFromHandle(windows::Win32::Foundation::HWND(handle as *mut _))?
        })
    }

    pub fn webauthn_windows(&self) -> Result<Vec<usize>> {
        let mut matches = Vec::new();
        for handle in self.windows_security_handles()? {
            let root = self.element_from_handle(handle)?;
            let elements = unsafe {
                root.FindAll(
                    TreeScope_Descendants,
                    &self.automation.CreateTrueCondition()?,
                )?
            };
            let mut names = Vec::new();
            for index in 0..unsafe { elements.Length()? } {
                names.push(unsafe { elements.GetElement(index)?.CurrentName()? }.to_string());
            }
            if super::webauthn_dialog_matches::webauthn_dialog_matches(&names) {
                matches.push(handle);
            }
        }
        Ok(matches)
    }

    pub fn webauthn_window(&self) -> Result<IUIAutomationElement> {
        self.wait("Windows WebAuthn dialog", || self.find_webauthn_window())
    }

    fn find_webauthn_window(&self) -> Result<Option<IUIAutomationElement>> {
        let handles = self.webauthn_windows()?;
        ensure!(
            handles.len() <= 1,
            "expected one WebAuthn dialog, found {}",
            handles.len()
        );
        let Some(handle) = handles.first() else {
            return Ok(None);
        };
        let root = self.element_from_handle(*handle)?;
        ensure!(
            unsafe { root.CurrentName()?.to_string() } == "Windows Security",
            "matched WebAuthn window is not Windows Security"
        );
        Ok(Some(root))
    }

    pub fn wait_webauthn_closed(&self) -> Result<()> {
        self.wait("Windows WebAuthn dialog closed", || {
            Ok(self.webauthn_windows()?.is_empty().then_some(()))
        })
    }

    pub fn webauthn_pin_editor_count(&self) -> Result<i32> {
        let handles = self.webauthn_windows()?;
        ensure!(handles.len() <= 1, "multiple WebAuthn dialogs for keyringd");
        match handles.first() {
            Some(_) => self.security_edit_count(&self.webauthn_window()?),
            None => Ok(0),
        }
    }

    pub fn enter_webauthn_pin(&self, pin: &str) -> Result<()> {
        let root = self.wait("WebAuthn PIN editor", || {
            let Some(root) = self.find_webauthn_window()? else {
                return Ok(None);
            };
            Ok((self.security_edit_count(&root)? == 1).then_some(root))
        })?;
        let editor = unsafe { self.security_editors(&root)?.GetElement(0)? };
        ensure!(
            unsafe { editor.CurrentIsPassword()?.as_bool() },
            "WebAuthn PIN editor is not a password field"
        );
        self.type_security_edit(&root, 0, pin)?;
        self.press_element_enter(&root, &editor)
    }

    pub fn security_windows_with_description(&self, description: &str) -> Result<Vec<usize>> {
        Ok(self
            .security_dialogs_with_description(description)?
            .into_iter()
            .map(|(handle, _, _)| handle)
            .collect())
    }

    // A fresh batched observation is made per visible candidate window.
    // The exact key description is identity metadata even when its label
    // is marked offscreen. Rejection text must be visible, and retry editors
    // must be visible and enabled.
    // Password values are never requested.
    fn security_dialogs_with_description(
        &self,
        description: &str,
    ) -> Result<Vec<(usize, Vec<String>, i32)>> {
        let snapshot = unsafe {
            let cache = self.automation.CreateCacheRequest()?;
            for property in [
                UIA_NamePropertyId,
                UIA_ControlTypePropertyId,
                UIA_IsOffscreenPropertyId,
                UIA_IsEnabledPropertyId,
            ] {
                cache.AddProperty(property)?;
            }
            cache
        };
        let mut matches = Vec::new();
        for handle in self.windows_security_handles()? {
            let root = self.element_from_handle(handle)?;
            let (description_matches, names, fields) = unsafe {
                let elements = root.FindAllBuildCache(
                    TreeScope_Descendants,
                    &self.automation.CreateTrueCondition()?,
                    &snapshot,
                )?;
                let mut description_matches = false;
                let mut names = Vec::new();
                let mut fields = 0;
                for index in 0..elements.Length()? {
                    let element = elements.GetElement(index)?;
                    if element.CachedControlType()? == UIA_EditControlTypeId {
                        if !element.CachedIsOffscreen()?.as_bool()
                            && element.CachedIsEnabled()?.as_bool()
                        {
                            fields += 1;
                        }
                    } else {
                        let name = element.CachedName()?.to_string();
                        description_matches |= name.lines().any(|line| {
                            line.trim() == description
                                || line
                                    .split_once(':')
                                    .is_some_and(|(_, value)| value.trim() == description)
                        });
                        if !element.CachedIsOffscreen()?.as_bool() {
                            names.push(name);
                        }
                    }
                }
                (description_matches, names, fields)
            };
            if description_matches {
                matches.push((handle, names, fields));
            }
        }
        Ok(matches)
    }

    fn find_security_window(&self, description: &str) -> Result<Option<usize>> {
        let matches = self.security_windows_with_description(description)?;
        ensure!(
            matches.len() <= 1,
            "expected one Windows Security dialog for {description:?}, found {}",
            matches.len()
        );
        Ok(matches.first().copied())
    }

    pub fn wait_security_window(&self, description: &str) -> Result<usize> {
        self.wait("Windows Security key description", || {
            self.find_security_window(description)
        })
    }

    pub fn wait_security_password_fields(
        &self,
        description: &str,
        expected: i32,
    ) -> Result<IUIAutomationElement> {
        self.wait("Windows CNG password fields", || {
            let Some(handle) = self.find_security_window(description)? else {
                return Ok(None);
            };
            let root = self.element_from_handle(handle)?;
            Ok((self.security_edit_count(&root)? == expected).then_some(root))
        })
    }

    pub fn wait_security_password_rejected(&self, description: &str) -> Result<usize> {
        self.wait("CNG password rejection", || {
            // Windows can replace the native dialog after a password attempt.
            let mut matches = self.security_dialogs_with_description(description)?;
            ensure!(
                matches.len() <= 1,
                "expected one Windows Security rejection dialog for {description:?}, found {}",
                matches.len()
            );
            let Some((handle, names, fields)) = matches.pop() else {
                return Ok(None);
            };
            Ok((fields == 1
                && names
                    .iter()
                    .any(|name| name.to_lowercase().contains("incorrect")))
            .then_some(handle))
        })
    }

    pub fn window_diagnostics(&self, handle: usize) -> String {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{
            GA_PARENT, GA_ROOT, GA_ROOTOWNER, GW_OWNER, GWLP_HWNDPARENT, GetAncestor,
            GetClassNameW, GetDesktopWindow, GetParent, GetWindow, GetWindowLongPtrW,
            GetWindowTextW, GetWindowThreadProcessId, IsWindow, IsWindowVisible,
        };
        let window = HWND(handle as *mut _);
        let mut pid = 0;
        let thread = unsafe { GetWindowThreadProcessId(window, Some(&mut pid)) };
        let mut title = [0u16; 256];
        let mut class = [0u16; 256];
        let title_length = unsafe { GetWindowTextW(window, &mut title) };
        let class_length = unsafe { GetClassNameW(window, &mut class) };
        format!(
            "HWND {handle}, PID {pid}, TID {thread}, valid={}, visible={}, class={:?}, title={:?}, GW_OWNER={:?}, GetParent={:?}, GA_ROOTOWNER={:?}, GA_PARENT={}, GA_ROOT={}, GWLP_HWNDPARENT={}, desktop_HWND={}",
            unsafe { IsWindow(Some(window)) }.as_bool(),
            unsafe { IsWindowVisible(window) }.as_bool(),
            String::from_utf16_lossy(&class[..class_length as usize]),
            String::from_utf16_lossy(&title[..title_length as usize]),
            unsafe { GetWindow(window, GW_OWNER) }.map(|hwnd| hwnd.0 as usize),
            unsafe { GetParent(window) }.map(|hwnd| hwnd.0 as usize),
            unsafe { GetAncestor(window, GA_ROOTOWNER) }.0 as usize,
            unsafe { GetAncestor(window, GA_PARENT) }.0 as usize,
            unsafe { GetAncestor(window, GA_ROOT) }.0 as usize,
            unsafe { GetWindowLongPtrW(window, GWLP_HWNDPARENT) },
            unsafe { GetDesktopWindow() }.0 as usize
        )
    }
    pub fn window_named(&self, pid: u32, title: &str) -> Result<IUIAutomationElement> {
        self.wait(title, || self.find_window_named(pid, title))
    }

    pub(crate) fn find_window_named(
        &self,
        pid: u32,
        title: &str,
    ) -> Result<Option<IUIAutomationElement>> {
        crate::application::platform::windows::window_locator::named_window(pid, title)?
            .map(|window| self.element_from_handle(window.0 as usize))
            .transpose()
    }

    pub fn window_with_id(&self, pid: u32, id: &str) -> Result<IUIAutomationElement> {
        self.wait(id, || {
            let windows =
                crate::application::platform::windows::window_locator::visible_windows(pid)?;
            let walker = unsafe { self.automation.RawViewWalker()? };
            let condition = unsafe {
                self.automation
                    .CreatePropertyCondition(UIA_AutomationIdPropertyId, &VARIANT::from(id))?
            };
            let mut found = None;
            for window in &windows {
                let root = self.element_from_handle(window.0 as usize)?;
                let elements = unsafe { root.FindAll(TreeScope_Descendants, &condition)? };
                for index in 0..unsafe { elements.Length()? } {
                    let mut element = unsafe { elements.GetElement(index)? };
                    // An owned dialog also appears below its owner in the UIA
                    // tree. Resolve each match to its nearest native top-level
                    // window so that the same dialog is not counted twice.
                    let containing_window = loop {
                        let handle = unsafe { element.CurrentNativeWindowHandle()? };
                        if windows.contains(&handle) {
                            break handle;
                        }
                        element = unsafe { walker.GetParentElement(&element)? };
                    };
                    ensure!(
                        found.is_none_or(|previous| previous == containing_window),
                        "multiple windows contain {id:?}"
                    );
                    found = Some(containing_window);
                }
            }
            found
                .map(|window| self.element_from_handle(window.0 as usize))
                .transpose()
        })
    }

    pub fn owner_window(&self, window: &IUIAutomationElement) -> Result<IUIAutomationElement> {
        use windows::Win32::UI::WindowsAndMessaging::{GW_OWNER, GetWindow};
        let owner = unsafe { GetWindow(window.CurrentNativeWindowHandle()?, GW_OWNER)? };
        ensure!(!owner.0.is_null(), "window has no native owner");
        self.element_from_handle(owner.0 as usize)
    }

    pub fn element_named(
        &self,
        root: &IUIAutomationElement,
        name: &str,
    ) -> Result<IUIAutomationElement> {
        self.wait(name, || self.find_named(root, name))
    }

    fn find_named(
        &self,
        root: &IUIAutomationElement,
        name: &str,
    ) -> Result<Option<IUIAutomationElement>> {
        unsafe {
            let condition = self
                .automation
                .CreatePropertyCondition(UIA_NamePropertyId, &VARIANT::from(name))?;
            let elements = root.FindAll(TreeScope_Descendants, &condition)?;
            if elements.Length()? == 0 {
                Ok(None)
            } else {
                Ok(Some(elements.GetElement(0)?))
            }
        }
    }

    pub fn contains_visible_named(&self, root: &IUIAutomationElement, name: &str) -> Result<bool> {
        unsafe {
            let named = self
                .automation
                .CreatePropertyCondition(UIA_NamePropertyId, &VARIANT::from(name))?;
            let visible = self
                .automation
                .CreatePropertyCondition(UIA_IsOffscreenPropertyId, &VARIANT::from(false))?;
            let condition = self.automation.CreateAndCondition(&named, &visible)?;
            Ok(root.FindAll(TreeScope_Descendants, &condition)?.Length()? > 0)
        }
    }

    pub fn wait_visible_named(&self, root: &IUIAutomationElement, name: &str) -> Result<()> {
        self.wait(name, || {
            Ok(self.contains_visible_named(root, name)?.then_some(()))
        })
    }

    pub fn wait_enabled_named(&self, root: &IUIAutomationElement, name: &str) -> Result<()> {
        self.wait(name, || {
            let Some(element) = self.find_named(root, name)? else {
                return Ok(None);
            };
            Ok(unsafe {
                (!element.CurrentIsOffscreen()?.as_bool() && element.CurrentIsEnabled()?.as_bool())
                    .then_some(())
            })
        })
    }

    pub fn visible_texts(&self, root: &IUIAutomationElement) -> Result<Vec<String>> {
        unsafe {
            let condition = self
                .automation
                .CreatePropertyCondition(
                    UIA_ControlTypePropertyId,
                    &VARIANT::from(UIA_TextControlTypeId.0),
                )
                .context("create visible texts ControlType condition")?;
            let elements = root
                .FindAll(TreeScope_Descendants, &condition)
                .context("find text descendants for visible texts")?;
            let length = elements
                .Length()
                .context("read visible texts element count")?;
            let mut texts = Vec::new();
            for index in 0..length {
                let element = elements
                    .GetElement(index)
                    .with_context(|| format!("read text element {index}/{length}"))?;
                if !element
                    .CurrentIsOffscreen()
                    .with_context(|| format!("read text IsOffscreen {index}/{length}"))?
                    .as_bool()
                {
                    let text = element
                        .CurrentName()
                        .with_context(|| format!("read visible text Name {index}/{length}"))?
                        .to_string();
                    if !text.is_empty() {
                        texts.push(text);
                    }
                }
            }
            Ok(texts)
        }
    }

    /// Read the visible value next to a labeled detail field.
    pub fn detail_value(&self, root: &IUIAutomationElement, field: &str) -> Result<String> {
        self.wait(field, || unsafe {
            let Some(label) = self.find_named(root, field)? else {
                return Ok(None);
            };
            if label.CurrentIsOffscreen()?.as_bool() {
                return Ok(None);
            }
            let value = match self
                .automation
                .ControlViewWalker()?
                .GetNextSiblingElement(&label)
            {
                Ok(value) => value,
                Err(error) if error.code() == windows::Win32::Foundation::E_POINTER => {
                    return Ok(None);
                }
                Err(error) => return Err(error.into()),
            };
            if value.CurrentIsOffscreen()?.as_bool() {
                return Ok(None);
            }
            ensure!(
                value.CurrentControlType()? == UIA_TextControlTypeId,
                "{field:?} is not followed by a detail text value"
            );
            let text = value.CurrentName()?.to_string();
            Ok((!text.is_empty()).then_some(text))
        })
    }

    pub fn wait_visible(&self, root: &IUIAutomationElement, id: &str) -> Result<()> {
        self.wait(id, || Ok(self.visible_element_by_id(root, id)?.map(|_| ())))
    }

    pub fn reveal_at_bottom(&self, root: &IUIAutomationElement, id: &str) -> Result<()> {
        self.reveal_at_bottom_until_visible(root, id, || {
            Ok(self.visible_element_by_id(root, id)?.is_some())
        })
    }

    pub fn reveal_named_at_bottom(&self, root: &IUIAutomationElement, name: &str) -> Result<()> {
        self.reveal_at_bottom_until_visible(root, name, || self.contains_visible_named(root, name))
    }

    fn reveal_at_bottom_until_visible(
        &self,
        root: &IUIAutomationElement,
        description: &str,
        visible: impl Fn() -> Result<bool>,
    ) -> Result<()> {
        self.wait(description, || unsafe {
            if visible()? {
                return Ok(Some(()));
            }
            let condition = self.automation.CreatePropertyCondition(
                UIA_IsScrollPatternAvailablePropertyId,
                &VARIANT::from(true),
            )?;
            let elements = root.FindAll(TreeScope_Descendants, &condition)?;
            if elements.Length()? == 0 {
                return Ok(None);
            }
            let scroll = elements
                .GetElement(0)?
                .GetCurrentPatternAs::<IUIAutomationScrollPattern>(UIA_ScrollPatternId)?;
            if scroll.CurrentVerticallyScrollable()?.as_bool() {
                scroll.SetScrollPercent(UIA_ScrollPatternNoScroll, 100.0)?;
            }
            Ok(None)
        })
    }

    pub fn invoke_named(&self, root: &IUIAutomationElement, name: &str) -> Result<()> {
        let element = self.element_named(root, name)?;
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(
                root.CurrentNativeWindowHandle()?,
            );
            ensure!(element.CurrentIsEnabled()?.as_bool(), "{name} is disabled");
            element
                .GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)?
                .Invoke()?;
        }
        Ok(())
    }

    pub(crate) fn wait_window_destroyed(
        &self,
        window: windows::Win32::Foundation::HWND,
    ) -> Result<()> {
        self.wait("native window destroyed", || {
            Ok(
                (!unsafe { windows::Win32::UI::WindowsAndMessaging::IsWindow(Some(window)) }
                    .as_bool())
                .then_some(()),
            )
        })
    }

    pub fn wait_window_absent(&self, pid: u32, title: &str) -> Result<()> {
        self.wait("window closed", || {
            Ok(
                crate::application::platform::windows::window_locator::named_window(pid, title)?
                    .is_none()
                    .then_some(()),
            )
        })
    }

    pub fn window_containing(
        &self,
        root: &IUIAutomationElement,
        name: &str,
    ) -> Result<IUIAutomationElement> {
        self.element_window(self.element_named(root, name)?)
    }

    fn element_window(&self, mut element: IUIAutomationElement) -> Result<IUIAutomationElement> {
        unsafe {
            let walker = self.automation.ControlViewWalker()?;
            for _ in 0..16 {
                if element.CurrentControlType()? == UIA_WindowControlTypeId {
                    return Ok(element);
                }
                element = walker.GetParentElement(&element)?;
            }
        }
        bail!("element has no window ancestor")
    }

    pub fn window_with_text(&self, pid: u32, name: &str) -> Result<IUIAutomationElement> {
        self.wait(name, || unsafe {
            for window in
                crate::application::platform::windows::window_locator::visible_windows(pid)?
            {
                let root = self.element_from_handle(window.0 as usize)?;
                if let Some(element) = self.find_named(&root, name)? {
                    if root.CurrentControlType()? == UIA_MenuControlTypeId {
                        return Ok(Some(root));
                    }
                    return Ok(Some(self.element_window(element)?));
                }
            }
            Ok(None)
        })
        .map_err(|error| error.context(self.diagnose_windows(pid)))
    }

    fn find_application_element_named(
        &self,
        pid: u32,
        name: &str,
    ) -> Result<Option<IUIAutomationElement>> {
        unsafe {
            let named = self
                .automation
                .CreatePropertyCondition(UIA_NamePropertyId, &VARIANT::from(name))?;
            for window in
                crate::application::platform::windows::window_locator::visible_windows(pid)?
            {
                let root = self.automation.ElementFromHandle(window)?;
                let elements = root.FindAll(TreeScope_Descendants, &named)?;
                for index in 0..elements.Length()? {
                    let element = elements.GetElement(index)?;
                    if !element.CurrentIsOffscreen()?.as_bool() {
                        return Ok(Some(element));
                    }
                }
            }
            Ok(None)
        }
    }

    fn application_element_named(&self, pid: u32, name: &str) -> Result<IUIAutomationElement> {
        self.wait(name, || self.find_application_element_named(pid, name))
    }

    pub fn wait_application_absent_named(&self, pid: u32, name: &str) -> Result<()> {
        self.wait(name, || {
            Ok(self
                .find_application_element_named(pid, name)?
                .is_none()
                .then_some(()))
        })
    }

    pub fn invoke_application_named(&self, pid: u32, name: &str) -> Result<()> {
        let element = self.application_element_named(pid, name)?;
        unsafe {
            element
                .GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId)?
                .Invoke()?;
        }
        Ok(())
    }

    pub fn application_element_enabled_named(&self, pid: u32, name: &str) -> Result<bool> {
        let element = self.application_element_named(pid, name)?;
        Ok(unsafe { element.CurrentIsEnabled()?.as_bool() })
    }

    pub fn dismiss_popup(&self, root: &IUIAutomationElement) -> Result<()> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VK_ESCAPE,
        };
        self.activate_input_target(root)?;
        let inputs = [
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_ESCAPE,
                        ..Default::default()
                    },
                },
            },
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VK_ESCAPE,
                        dwFlags: KEYEVENTF_KEYUP,
                        ..Default::default()
                    },
                },
            },
        ];
        ensure!(
            unsafe { SendInput(&inputs, size_of::<INPUT>() as i32) } == 2,
            "could not dismiss popup"
        );
        Ok(())
    }

    pub fn select_named(&self, root: &IUIAutomationElement, name: &str) -> Result<()> {
        let element = self.element_named(root, name)?;
        unsafe {
            let selection = element.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                UIA_SelectionItemPatternId,
            )?;
            selection.Select()?;
            self.wait("selected passkey", || {
                Ok(selection.CurrentIsSelected()?.as_bool().then_some(()))
            })
        }
    }

    pub fn select_option(&self, root: &IUIAutomationElement, id: &str, index: i32) -> Result<()> {
        let group = self.element(root, id)?;
        unsafe {
            let condition = self.automation.CreatePropertyCondition(
                UIA_ControlTypePropertyId,
                &VARIANT::from(UIA_RadioButtonControlTypeId.0),
            )?;
            let option = group
                .FindAll(TreeScope_Descendants, &condition)?
                .GetElement(index)?;
            let selection = option.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                UIA_SelectionItemPatternId,
            )?;
            selection.Select()?;
            self.wait("selected option", || {
                Ok(selection.CurrentIsSelected()?.as_bool().then_some(()))
            })
        }
    }

    pub fn select_item(&self, root: &IUIAutomationElement, id: &str) -> Result<()> {
        let element = self.element(root, id)?;
        unsafe {
            let selection = element.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(
                UIA_SelectionItemPatternId,
            )?;
            selection.Select()?;
            self.wait("selected item", || {
                Ok(selection.CurrentIsSelected()?.as_bool().then_some(()))
            })
        }
    }

    pub fn text(&self, root: &IUIAutomationElement, id: &str) -> Result<String> {
        let element = self.element(root, id)?;
        unsafe {
            Ok(element
                .GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)?
                .DocumentRange()?
                .GetText(-1)?
                .to_string()
                .trim_end_matches(['\r', '\n'])
                .to_owned())
        }
    }

    /// InvokePattern waits for the WinUI handler, which can block in a native
    /// TaskDialog. Keyboard activation lets its modal loop run independently.
    pub fn press_application_enter(
        &self,
        pid: u32,
        root: &IUIAutomationElement,
        name: &str,
    ) -> Result<()> {
        let element = self.application_element_named(pid, name)?;
        self.press_element_enter(root, &element)
    }

    pub fn press_enter(&self, root: &IUIAutomationElement, id: &str) -> Result<()> {
        self.press_element_enter(root, &self.element(root, id)?)
    }

    fn press_element_enter(
        &self,
        root: &IUIAutomationElement,
        element: &IUIAutomationElement,
    ) -> Result<()> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, SendInput, VK_RETURN,
        };
        unsafe {
            self.activate_input_target(root)?;
            element.SetFocus()?;
            self.wait("element keyboard focus", || {
                Ok(element.CurrentHasKeyboardFocus()?.as_bool().then_some(()))
            })?;
            let inputs = [
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_RETURN,
                            ..Default::default()
                        },
                    },
                },
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_RETURN,
                            dwFlags: KEYEVENTF_KEYUP,
                            ..Default::default()
                        },
                    },
                },
            ];
            ensure!(
                SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) == 2,
                "could not send Enter to element"
            );
        }
        Ok(())
    }

    fn security_editors(&self, root: &IUIAutomationElement) -> Result<IUIAutomationElementArray> {
        unsafe {
            let edit = self.automation.CreatePropertyCondition(
                UIA_ControlTypePropertyId,
                &VARIANT::from(UIA_EditControlTypeId.0),
            )?;
            let visible = self
                .automation
                .CreatePropertyCondition(UIA_IsOffscreenPropertyId, &VARIANT::from(false))?;
            let enabled = self
                .automation
                .CreatePropertyCondition(UIA_IsEnabledPropertyId, &VARIANT::from(true))?;
            let condition = self.automation.CreateAndCondition(&edit, &visible)?;
            let condition = self.automation.CreateAndCondition(&condition, &enabled)?;
            Ok(root.FindAll(TreeScope_Descendants, &condition)?)
        }
    }

    pub fn security_edit_count(&self, root: &IUIAutomationElement) -> Result<i32> {
        Ok(unsafe { self.security_editors(root)?.Length()? })
    }

    pub fn type_security_edit(
        &self,
        root: &IUIAutomationElement,
        index: i32,
        value: &str,
    ) -> Result<()> {
        let element = unsafe { self.security_editors(root)?.GetElement(index)? };
        unsafe {
            element
                .GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)?
                .SetValue(&BSTR::from(value))?;
        }
        Ok(())
    }
    pub fn submit_security_dialog(&self, root: &IUIAutomationElement) -> Result<()> {
        // InvokePattern can wait for the native modal handler to return.
        // Activate the identified button asynchronously, then observe the outcome.
        self.press_enter(root, "OkButton")
    }
    pub fn clipboard_sequence(&self) -> u32 {
        unsafe { windows::Win32::System::DataExchange::GetClipboardSequenceNumber() }
    }
    pub fn clipboard_after(&self, sequence: u32) -> Result<String> {
        self.wait("clipboard write", || {
            if self.clipboard_sequence() == sequence {
                return Ok(None);
            }
            let Some(clipboard) =
                crate::application::platform::windows::ClipboardSession::try_open()?
            else {
                return Ok(None);
            };
            Ok(Some(clipboard.text()?))
        })
    }

    pub fn drag_before(
        &self,
        root: &IUIAutomationElement,
        source: &str,
        target: &str,
    ) -> Result<()> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN,
            MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT, SendInput,
        };
        use windows::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
            SM_YVIRTUALSCREEN,
        };
        let source = self.element_named(root, source)?;
        let target = self.element_named(root, target)?;
        unsafe {
            let walker = self.automation.ControlViewWalker()?;
            let row = walker.GetParentElement(&source)?;
            let grip = self.element_named(&row, "⠿")?.CurrentBoundingRectangle()?;
            let target = target.CurrentBoundingRectangle()?;
            let x = (grip.left + grip.right) / 2;
            let start_y = (grip.top + grip.bottom) / 2;
            let end_y = target.top;
            self.activate_input_target(root)?;
            let send = |flags, point: Option<(i32, i32)>| -> Result<()> {
                let (dx, dy) = point
                    .map(|(x, y)| {
                        (
                            ((i64::from(x - GetSystemMetrics(SM_XVIRTUALSCREEN)) * 65535)
                                / i64::from(GetSystemMetrics(SM_CXVIRTUALSCREEN) - 1))
                                as i32,
                            ((i64::from(y - GetSystemMetrics(SM_YVIRTUALSCREEN)) * 65535)
                                / i64::from(GetSystemMetrics(SM_CYVIRTUALSCREEN) - 1))
                                as i32,
                        )
                    })
                    .unwrap_or_default();
                let input = INPUT {
                    r#type: INPUT_MOUSE,
                    Anonymous: INPUT_0 {
                        mi: MOUSEINPUT {
                            dx,
                            dy,
                            dwFlags: flags,
                            ..Default::default()
                        },
                    },
                };
                ensure!(
                    SendInput(&[input], std::mem::size_of::<INPUT>() as i32) == 1,
                    "mouse input failed"
                );
                Ok(())
            };
            let movement = MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK;
            send(movement, Some((x, start_y)))?;
            send(MOUSEEVENTF_LEFTDOWN, None)?;
            std::thread::sleep(Duration::from_millis(60));
            let motion = (|| -> Result<()> {
                for step in 1..=16 {
                    send(movement, Some((x, start_y + (end_y - start_y) * step / 16)))?;
                    // This is pointer gesture pacing, not a wait for application state.
                    std::thread::sleep(Duration::from_millis(16));
                }
                Ok(())
            })();
            std::thread::sleep(Duration::from_millis(60));
            let released = send(MOUSEEVENTF_LEFTUP, None);
            motion?;
            released?;
        }
        Ok(())
    }

    pub fn element_is_above(
        &self,
        root: &IUIAutomationElement,
        first: &str,
        second: &str,
    ) -> Result<bool> {
        unsafe {
            let first = self
                .element_named(root, first)?
                .CurrentBoundingRectangle()?;
            let second = self
                .element_named(root, second)?
                .CurrentBoundingRectangle()?;
            Ok(first.top < second.top)
        }
    }

    /// SendInput goes to the foreground window, regardless of the UIA element
    /// used to locate the control. Refuse to inject into another process.
    pub(crate) fn activate_input_target(&self, root: &IUIAutomationElement) -> Result<()> {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId, SetForegroundWindow,
        };
        let target = unsafe { root.CurrentNativeWindowHandle()? };
        let mut target_pid = 0;
        unsafe { GetWindowThreadProcessId(target, Some(&mut target_pid)) };
        ensure!(target_pid != 0, "input target has no process");
        self.wait("input target foreground", || {
            let _ = unsafe { SetForegroundWindow(target) };
            let foreground = unsafe { GetForegroundWindow() };
            let mut foreground_pid = 0;
            unsafe { GetWindowThreadProcessId(foreground, Some(&mut foreground_pid)) };
            Ok((foreground_pid == target_pid && foreground == target).then_some(()))
        })
    }

    pub fn toggle_named(&self, root: &IUIAutomationElement, name: &str) -> Result<()> {
        let element = self.element_named(root, name)?;
        unsafe {
            element
                .GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId)?
                .Toggle()?;
        }
        Ok(())
    }

    pub fn toggle_state_named(&self, root: &IUIAutomationElement, name: &str) -> Result<bool> {
        unsafe {
            let element = self.element_named(root, name)?;
            let state = element
                .GetCurrentPatternAs::<IUIAutomationTogglePattern>(UIA_TogglePatternId)?
                .CurrentToggleState()?;
            ensure!(
                state == ToggleState_On || state == ToggleState_Off,
                "indeterminate toggle state for {name}"
            );
            Ok(state == ToggleState_On)
        }
    }

    /// Captures the target window's compositor surface after its UI and pixels settle.
    pub fn capture(&self, root: &IUIAutomationElement) -> Result<PathBuf> {
        let hwnd = unsafe { root.CurrentNativeWindowHandle()? }.0 as usize;
        ensure!(hwnd != 0, "screenshot target has no native window handle");
        let window = windows::Win32::Foundation::HWND(hwnd as *mut _);
        let directory = PathBuf::from(
            std::env::var_os("KEYRINGD_TEST_REPORT_DIR")
                .context("report directory is required for screenshots")?,
        )
        .join("screenshots");
        std::fs::create_dir_all(&directory)?;
        let filename = format!("{}.png", uuid::Uuid::new_v4());
        let path = directory.join(&filename);
        let _dpi = crate::application::platform::windows::DpiAwarenessGuard::enter()?;
        crate::application::platform::windows::WindowCapture::new(window)
            .with_context(|| {
                format!(
                    "could not start capture: UIA HWND={hwnd:#x}, {}",
                    self.window_diagnostics(window.0 as usize)
                )
            })?
            .save_when_ready(&path, || self.capture_ui_state(root))
            .context("failed to capture the target window")?;
        // Image evidence resolves relative to index.html, so the report stays portable.
        Ok(PathBuf::from("screenshots").join(filename))
    }

    fn capture_ui_state(&self, root: &IUIAutomationElement) -> Result<Vec<String>> {
        unsafe {
            let visible = self
                .automation
                .CreatePropertyCondition(UIA_IsOffscreenPropertyId, &VARIANT::from(false))?;
            let elements = root.FindAll(TreeScope_Descendants, &visible)?;
            let mut state = Vec::new();
            for index in 0..elements.Length()? {
                let element = elements.GetElement(index)?;
                let rect = element.CurrentBoundingRectangle()?;
                state.push(format!(
                    "{}|{}|{}|{},{},{},{}",
                    element.CurrentControlType()?.0,
                    element.CurrentAutomationId()?,
                    element.CurrentName()?,
                    rect.left,
                    rect.top,
                    rect.right,
                    rect.bottom
                ));
            }
            Ok(state)
        }
    }

    /// Waits for a Hybrid URI decoded from the displayed pixels, after the prior step's QR disappears.
    pub fn read_hybrid_qr(
        &self,
        root: &IUIAutomationElement,
        previous: Option<&str>,
    ) -> Result<(String, PathBuf)> {
        let report = PathBuf::from(
            std::env::var_os("KEYRINGD_TEST_REPORT_DIR")
                .context("report directory is required for QR decoding")?,
        );
        self.wait("rendered Hybrid QR", || {
            let screenshot = self.capture(root)?;
            let Some(uri) = keyringd_testkit::QrCodeReader::read_hybrid(&report.join(&screenshot))?
            else {
                return Ok(None);
            };
            Ok((previous != Some(uri.as_str())).then_some((uri, screenshot)))
        })
    }

    pub fn visible_element_by_id(
        &self,
        root: &IUIAutomationElement,
        id: &str,
    ) -> Result<Option<IUIAutomationElement>> {
        unsafe {
            let named = self
                .automation
                .CreatePropertyCondition(UIA_AutomationIdPropertyId, &VARIANT::from(id))
                .with_context(|| format!("create AutomationId condition for {id:?}"))?;
            let visible = self
                .automation
                .CreatePropertyCondition(UIA_IsOffscreenPropertyId, &VARIANT::from(false))
                .with_context(|| format!("create IsOffscreen condition for {id:?}"))?;
            let condition = self
                .automation
                .CreateAndCondition(&named, &visible)
                .with_context(|| format!("combine visible AutomationId conditions for {id:?}"))?;
            let elements = root
                .FindAll(TreeScope_Descendants, &condition)
                .with_context(|| format!("find visible AutomationId descendants for {id:?}"))?;
            if elements
                .Length()
                .with_context(|| format!("read visible AutomationId count for {id:?}"))?
                == 0
            {
                Ok(None)
            } else {
                Ok(Some(elements.GetElement(0).with_context(|| {
                    format!("read first visible AutomationId element for {id:?}")
                })?))
            }
        }
    }

    pub fn element(&self, root: &IUIAutomationElement, id: &str) -> Result<IUIAutomationElement> {
        self.wait(id, || self.element_now(root, id))
    }

    pub fn set_value(&self, root: &IUIAutomationElement, id: &str, value: &str) -> Result<()> {
        let element = self.element(root, id)?;
        unsafe {
            let pattern: IUIAutomationValuePattern =
                element.GetCurrentPattern(UIA_ValuePatternId)?.cast()?;
            pattern.SetValue(&BSTR::from(value))?;
        }
        self.wait("editor value update", || unsafe {
            let Some(element) = self.element_now(root, id)? else {
                return Ok(None);
            };
            let pattern =
                element.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)?;
            Ok((pattern.CurrentValue()? == value).then_some(()))
        })
    }

    pub fn invoke(&self, root: &IUIAutomationElement, id: &str) -> Result<()> {
        let element = self.element(root, id)?;
        unsafe {
            if !element.CurrentIsEnabled()?.as_bool() {
                bail!("{id} is disabled");
            }
            let pattern: IUIAutomationInvokePattern =
                element.GetCurrentPattern(UIA_InvokePatternId)?.cast()?;
            pattern.Invoke()?;
        }
        Ok(())
    }

    pub fn expand(&self, root: &IUIAutomationElement, id: &str) -> Result<()> {
        let element = self.element(root, id)?;
        unsafe {
            element
                .GetCurrentPatternAs::<IUIAutomationExpandCollapsePattern>(
                    UIA_ExpandCollapsePatternId,
                )?
                .Expand()?;
        }
        Ok(())
    }

    pub fn wait_enabled(&self, root: &IUIAutomationElement, id: &str, enabled: bool) -> Result<()> {
        self.wait(id, || {
            let Some(element) = self.element_now(root, id)? else {
                return Ok(None);
            };
            Ok((unsafe { element.CurrentIsEnabled()?.as_bool() } == enabled).then_some(()))
        })
    }

    pub fn wait_window_enabled(&self, root: &IUIAutomationElement, enabled: bool) -> Result<()> {
        self.wait("window enabled state", || {
            Ok((unsafe { root.CurrentIsEnabled()?.as_bool() } == enabled).then_some(()))
        })
    }

    pub fn scroll_to_bottom(&self, root: &IUIAutomationElement) -> Result<()> {
        self.wait("scroll to bottom", || unsafe {
            let condition = self.automation.CreatePropertyCondition(
                UIA_IsScrollPatternAvailablePropertyId,
                &VARIANT::from(true),
            )?;
            let elements = root.FindAll(TreeScope_Descendants, &condition)?;
            if elements.Length()? == 0 {
                return Ok(None);
            }
            let scroll = elements
                .GetElement(0)?
                .GetCurrentPatternAs::<IUIAutomationScrollPattern>(UIA_ScrollPatternId)?;
            if !scroll.CurrentVerticallyScrollable()?.as_bool() {
                return Ok(None);
            }
            scroll.SetScrollPercent(UIA_ScrollPatternNoScroll, 100.0)?;
            Ok((scroll.CurrentVerticalScrollPercent()? >= 99.0).then_some(()))
        })
    }

    fn element_now(
        &self,
        root: &IUIAutomationElement,
        id: &str,
    ) -> Result<Option<IUIAutomationElement>> {
        unsafe {
            let condition = self
                .automation
                .CreatePropertyCondition(UIA_AutomationIdPropertyId, &VARIANT::from(id))?;
            let elements = root.FindAll(TreeScope_Descendants, &condition)?;
            if elements.Length()? == 0 {
                Ok(None)
            } else {
                Ok(Some(elements.GetElement(0)?))
            }
        }
    }

    /// Only an unmet observation is retried. Query failures stop the wait immediately.
    pub(crate) fn wait<T>(
        &self,
        label: &str,
        mut query: impl FnMut() -> Result<Option<T>>,
    ) -> Result<T> {
        let deadline = Instant::now() + TIMEOUT;
        loop {
            if let Some(value) = query().with_context(|| format!("could not observe {label}"))? {
                return Ok(value);
            }
            ensure!(Instant::now() < deadline, "timed out waiting for {label}");
            std::thread::sleep(
                Duration::from_millis(50).min(deadline.saturating_duration_since(Instant::now())),
            );
        }
    }
}
