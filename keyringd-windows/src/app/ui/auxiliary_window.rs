use crate::app::ui::util::{CreatedReactorWindowLocator, center_on_cursor_monitor};

use windows::Win32::Foundation::HWND;

use windows::Win32::UI::WindowsAndMessaging::{
    GWLP_HWNDPARENT, IsWindow, SetForegroundWindow, SetWindowLongPtrW,
};

use windows_reactor::{
    Backdrop, Element, InputExt, KeyboardAccelerator, ReactorWindow, RenderCx, VirtualKey,
    VirtualKeyModifiers, grid,
};

use crate::app::ui::util::ModalWindowLifecycle;
use std::{cell::Cell, rc::Rc};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    PostMessageW, SW_HIDE, SW_SHOW, ShowWindow, WM_CLOSE,
};

#[derive(Clone)]
pub(in crate::app::ui) struct AuxiliaryWindow {
    handle: Rc<Cell<usize>>,
    detached_parent: Rc<Cell<usize>>,
}

impl AuxiliaryWindow {
    pub(super) fn open_root(
        title: impl Into<String>,
        size: (f64, f64),
        render: impl Fn(&mut RenderCx, Self) -> Element + 'static,
    ) -> anyhow::Result<()> {
        Self::open_root_tracked(title, size, render).map(|_| ())
    }

    pub(in crate::app::ui) fn open_root_tracked(
        title: impl Into<String>,
        size: (f64, f64),
        render: impl Fn(&mut RenderCx, Self) -> Element + 'static,
    ) -> anyhow::Result<Self> {
        Self::open(title, size, None, render)
    }

    pub(in crate::app::ui) fn open_modal(
        title: impl Into<String>,
        size: (f64, f64),
        parent: usize,
        render: impl Fn(&mut RenderCx, Self) -> Element + 'static,
    ) -> anyhow::Result<()> {
        Self::open(title, size, Some(parent), render).map(|_| ())
    }

    fn open(
        title: impl Into<String>,
        size: (f64, f64),
        parent: Option<usize>,
        render: impl Fn(&mut RenderCx, Self) -> Element + 'static,
    ) -> anyhow::Result<Self> {
        let window = Self {
            handle: Rc::new(Cell::new(0)),
            detached_parent: Rc::new(Cell::new(0)),
        };
        let rendered = window.clone();
        let locator = CreatedReactorWindowLocator::begin()?;
        ReactorWindow::new()
            .title(title)
            .inner_size(size.0, size.1)
            .backdrop(Backdrop::Mica)
            .render(move |cx| render(cx, rendered.clone()))?;
        let handle = locator.finish()?;
        window.handle.set(handle.0 as usize);
        center_on_cursor_monitor(handle);
        crate::app::ui::util::apply_application_icon(handle)?;
        if let Some(parent) = parent.filter(|parent| *parent != 0) {
            ModalWindowLifecycle::attach(handle, HWND(parent as *mut _))?;
        }
        Ok(window)
    }

    pub(in crate::app::ui) fn hwnd(&self) -> usize {
        self.handle.get()
    }

    pub(super) fn hide(&self) {
        let hwnd = HWND(self.hwnd() as *mut _);
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
            // Keep an in-flight native operation's owner alive if the originating window closes.
            self.detached_parent
                .set(SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, 0) as usize);
        }
    }

    pub(super) fn show(&self) {
        let hwnd = HWND(self.hwnd() as *mut _);
        if unsafe { IsWindow(Some(hwnd)) }.as_bool() {
            unsafe {
                let parent = self.detached_parent.replace(0);
                if parent != 0 && IsWindow(Some(HWND(parent as *mut _))).as_bool() {
                    SetWindowLongPtrW(hwnd, GWLP_HWNDPARENT, parent as isize);
                }
                let _ = ShowWindow(hwnd, SW_SHOW);
                let _ = SetForegroundWindow(hwnd);
            }
        }
    }

    pub(in crate::app::ui) fn close(&self) {
        if self.hwnd() != 0 {
            unsafe {
                let _ = PostMessageW(
                    Some(HWND(self.hwnd() as *mut _)),
                    WM_CLOSE,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
        }
    }

    pub(in crate::app::ui) fn content(&self, content: Element, can_close: bool) -> Element {
        let window = self.clone();
        grid((content,))
            .keyboard_accelerator(KeyboardAccelerator::new(
                VirtualKey::Escape,
                VirtualKeyModifiers::None,
                move || {
                    if can_close {
                        window.close();
                    }
                },
            ))
            .into()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::app::ui::test_support::TestWindow;
    use windows::Win32::UI::Input::KeyboardAndMouse::IsWindowEnabled;
    use windows::Win32::UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetWindowLongPtrW, HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
        WS_EX_TOPMOST,
    };

    keyringd_testkit::scenario! {
        group "AuxiliaryWindow::open_modal" {
            case "Modal window stays above topmost owner" {
                verify "keep the modal above an owner that is already topmost" {
                    let parent = TestWindow::create();
                    let child = TestWindow::create();
                    unsafe {
                        SetWindowPos(
                            parent.0,
                            Some(HWND_TOPMOST),
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE,
                        )
                        .unwrap();
                        assert_ne!(
                            GetWindowLongPtrW(parent.0, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0,
                            0,
                            "test parent must be topmost before attaching the modal window"
                        );
                        assert_eq!(
                            GetWindowLongPtrW(child.0, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0,
                            0
                        );
                    }

                    ModalWindowLifecycle::attach(child.0, parent.0).unwrap();

                    unsafe {
                        assert_eq!(
                            GetWindowLongPtrW(child.0, GWLP_HWNDPARENT),
                            parent.0.0 as isize
                        );
                        assert_ne!(
                            GetWindowLongPtrW(child.0, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0,
                            0
                        );
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "AuxiliaryWindow::open_modal" {
            case "Set Owner to Modal Window and disable parent Window only while operating" {
                given "Independent parent window and child window exist" {
                    let parent = TestWindow::create();
                    let child = TestWindow::create();
                    assert!(unsafe { IsWindowEnabled(parent.0) }.as_bool());
                }
                when "Associate a child window as a modal to the parent window" {
                    ModalWindowLifecycle::attach(child.0, parent.0).unwrap();
                }
                then "Child Window keeps Owner and parent Window is disabled" {
                    assert_eq!(
                        unsafe { GetWindowLongPtrW(child.0, GWLP_HWNDPARENT) },
                        (parent.0).0 as isize
                    );
                    assert!(!unsafe { IsWindowEnabled(parent.0) }.as_bool());

                    drop(child);
                    assert!(unsafe { IsWindowEnabled(parent.0) }.as_bool());
                }
            }
        }
    }
}
