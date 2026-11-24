use std::{cell::RefCell, collections::HashMap};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, IsWindowEnabled};
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GWLP_HWNDPARENT, GetWindowLongPtrW, HWND_TOP, HWND_TOPMOST, IsWindow, SWP_NOMOVE,
    SWP_NOOWNERZORDER, SWP_NOSIZE, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos,
    WM_NCDESTROY, WS_EX_TOPMOST,
};

/// Owns a native modal relationship until the child's actual destruction.
/// Render effects and component cleanup do not control the parent's lifetime.
pub(crate) struct ModalWindowLifecycle {
    owner: HWND,
}

thread_local! {
    // Native modal windows are created and destroyed on their owner's UI thread.
    // Preserve the original enabled state across overlapping sibling windows.
    static MODAL_OWNERS: RefCell<HashMap<usize, (usize, bool)>> = RefCell::new(HashMap::new());
}

impl ModalWindowLifecycle {
    pub(crate) fn attach(child: HWND, owner: HWND) -> anyhow::Result<()> {
        if owner.0.is_null() {
            return Ok(());
        }
        unsafe {
            anyhow::ensure!(
                IsWindow(Some(owner)).as_bool(),
                "modal owner is unavailable"
            );
            SetWindowLongPtrW(child, GWLP_HWNDPARENT, owner.0 as isize);
            let band = if GetWindowLongPtrW(owner, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0 {
                HWND_TOPMOST
            } else {
                HWND_TOP
            };
            SetWindowPos(
                child,
                Some(band),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOOWNERZORDER,
            )?;
            let lifecycle = Box::new(Self { owner });
            let state = Box::into_raw(lifecycle);
            if !SetWindowSubclass(
                child,
                Some(modal_window_subclass),
                modal_window_subclass as *const () as usize,
                state as usize,
            )
            .as_bool()
            {
                drop(Box::from_raw(state));
                anyhow::bail!("could not observe modal window destruction");
            }
            MODAL_OWNERS.with_borrow_mut(|owners| {
                let entry = owners
                    .entry(owner.0 as usize)
                    .or_insert_with(|| (0, IsWindowEnabled(owner).as_bool()));
                entry.0 += 1;
            });
            let _ = EnableWindow(owner, false);
            let _ = SetForegroundWindow(child);
        }
        Ok(())
    }

    fn closed(self) {
        let restore_enabled = MODAL_OWNERS.with_borrow_mut(|owners| {
            let entry = owners.get_mut(&(self.owner.0 as usize))?;
            entry.0 -= 1;
            if entry.0 > 0 {
                return None;
            }
            owners.remove(&(self.owner.0 as usize)).map(|entry| entry.1)
        });
        let Some(enabled) = restore_enabled else {
            return;
        };
        unsafe {
            if IsWindow(Some(self.owner)).as_bool() {
                let _ = EnableWindow(self.owner, enabled);
                if enabled {
                    let _ = SetForegroundWindow(self.owner);
                }
            }
        }
    }
}

unsafe extern "system" fn modal_window_subclass(
    child: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    subclass_id: usize,
    state: usize,
) -> LRESULT {
    unsafe {
        if message == WM_NCDESTROY {
            let _ = RemoveWindowSubclass(child, Some(modal_window_subclass), subclass_id);
            let lifecycle = Box::from_raw(state as *mut ModalWindowLifecycle);
            let result = DefSubclassProc(child, message, wparam, lparam);
            lifecycle.closed();
            result
        } else {
            DefSubclassProc(child, message, wparam, lparam)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ui::test_support::TestWindow;

    keyringd_testkit::scenario! {
        group "ModalWindowLifecycle::attach" {
            case "Closing an old modal does not enable the owner of its replacement" {
                verify "keep the owner disabled while a replacement modal is still open" {
                    let parent = TestWindow::create();
                    let old = TestWindow::create();
                    ModalWindowLifecycle::attach(old.0, parent.0).unwrap();
                    let replacement = TestWindow::create();
                    ModalWindowLifecycle::attach(replacement.0, parent.0).unwrap();
                    drop(old);
                    assert!(!unsafe { IsWindowEnabled(parent.0) }.as_bool());
                    drop(replacement);
                    assert!(unsafe { IsWindowEnabled(parent.0) }.as_bool());
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "ModalWindowLifecycle::attach" {
            case "Initially disabled owner stays disabled after its last modal closes" {
                verify "preserve the owner's original disabled state after the final modal closes" {
                    let parent = TestWindow::create();
                    unsafe {
                        let _ = EnableWindow(parent.0, false);
                    }
                    let first = TestWindow::create();
                    let second = TestWindow::create();
                    ModalWindowLifecycle::attach(first.0, parent.0).unwrap();
                    ModalWindowLifecycle::attach(second.0, parent.0).unwrap();
                    drop(second);
                    assert!(!unsafe { IsWindowEnabled(parent.0) }.as_bool());
                    drop(first);
                    assert!(!unsafe { IsWindowEnabled(parent.0) }.as_bool());
                }
            }
        }
    }
}
