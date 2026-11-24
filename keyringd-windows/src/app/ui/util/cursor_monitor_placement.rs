use windows::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint,
};
use windows::Win32::UI::Controls::{TASKDIALOG_NOTIFICATIONS, TDN_CREATED};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetWindowRect, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_NOZORDER,
    SetWindowPos,
};
use windows::core::HRESULT;

pub(crate) fn center_on_cursor_monitor(window: HWND) {
    let mut cursor = POINT::default();
    if unsafe { GetCursorPos(&mut cursor) }.is_err() {
        return;
    }
    center_on_monitor_at(window, cursor);
}

fn center_on_monitor_at(window: HWND, point: POINT) {
    let monitor = unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return;
    }

    // Crossing to a monitor with a different DPI can resize the window.
    // Re-read its bounds once to center it at its new size.
    for _ in 0..2 {
        let mut bounds = RECT::default();
        if unsafe { GetWindowRect(window, &mut bounds) }.is_err() {
            return;
        }
        let position = centered_position(info.rcWork, bounds);
        if position.x == bounds.left && position.y == bounds.top {
            return;
        }
        if let Err(error) = unsafe {
            SetWindowPos(
                window,
                None,
                position.x,
                position.y,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER | SWP_NOOWNERZORDER,
            )
        } {
            tracing::warn!(%error, "could not position the window on the cursor monitor");
            return;
        }
    }
}

pub(crate) extern "system" fn place_task_dialog_on_cursor_monitor(
    window: HWND,
    notification: TASKDIALOG_NOTIFICATIONS,
    _wparam: WPARAM,
    _lparam: LPARAM,
    _data: isize,
) -> HRESULT {
    if notification == TDN_CREATED {
        center_on_cursor_monitor(window);
    }
    HRESULT(0)
}

fn centered_position(work_area: RECT, bounds: RECT) -> POINT {
    let width = bounds.right - bounds.left;
    let height = bounds.bottom - bounds.top;
    POINT {
        x: work_area.left + (work_area.right - work_area.left - width).max(0) / 2,
        y: work_area.top + (work_area.bottom - work_area.top - height).max(0) / 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ui::test_support::TestWindow;
    use crate::app::ui::util::ModalWindowLifecycle;
    use windows::Win32::Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR, MonitorFromWindow};
    use windows::core::BOOL;

    #[test]
    fn opens_an_owned_window_on_each_monitor_without_moving_the_cursor() {
        let mut monitor_centers: Vec<POINT> = Vec::new();
        let enumerated = unsafe {
            EnumDisplayMonitors(
                None,
                None,
                Some(collect_monitor_center),
                LPARAM(std::ptr::from_mut(&mut monitor_centers) as isize),
            )
        };
        assert!(enumerated.as_bool());
        assert!(!monitor_centers.is_empty());
        let owner = TestWindow::create();
        for &point in &monitor_centers {
            let window = TestWindow::create();
            center_on_monitor_at(window.0, point);
            ModalWindowLifecycle::attach(window.0, owner.0).unwrap();
            let expected = unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) };
            let actual = unsafe { MonitorFromWindow(window.0, MONITOR_DEFAULTTONEAREST) };
            assert_eq!(actual, expected);
        }
        println!(
            "Verified placement on {} display monitors",
            monitor_centers.len()
        );
    }

    unsafe extern "system" fn collect_monitor_center(
        _monitor: HMONITOR,
        _dc: HDC,
        bounds: *mut RECT,
        context: LPARAM,
    ) -> BOOL {
        let bounds = unsafe { *bounds };
        let centers = unsafe { &mut *(context.0 as *mut Vec<POINT>) };
        centers.push(POINT {
            x: bounds.left + (bounds.right - bounds.left) / 2,
            y: bounds.top + (bounds.bottom - bounds.top) / 2,
        });
        true.into()
    }

    #[test]
    fn centers_in_the_work_area_of_a_monitor_left_and_above_the_primary() {
        let work_area = RECT {
            left: -1920,
            top: -900,
            right: 0,
            bottom: 140,
        };
        let bounds = RECT {
            left: 100,
            top: 100,
            right: 780,
            bottom: 500,
        };
        assert_eq!(
            centered_position(work_area, bounds),
            POINT { x: -1300, y: -580 }
        );
    }

    #[test]
    fn keeps_the_title_bar_on_the_monitor_when_the_window_is_too_large() {
        let work_area = RECT {
            left: 1920,
            top: 40,
            right: 2560,
            bottom: 520,
        };
        let bounds = RECT {
            left: 0,
            top: 0,
            right: 700,
            bottom: 640,
        };
        assert_eq!(
            centered_position(work_area, bounds),
            POINT { x: 1920, y: 40 }
        );
    }
}
