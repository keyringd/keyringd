use tray_icon::Icon;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{LIM_SMALL, LoadIconMetric};
use windows::Win32::UI::WindowsAndMessaging::{
    HICON, ICON_BIG, ICON_SMALL, LoadIconW, SendMessageW, WM_SETICON,
};
use windows::core::PCWSTR;

// Win32 encodes numeric resource IDs as pointers (MAKEINTRESOURCE).
const APPLICATION_ICON: PCWSTR = PCWSTR(std::ptr::without_provenance(1));

pub(crate) fn load_application_icon() -> windows::core::Result<HICON> {
    let instance = unsafe { GetModuleHandleW(None)? };
    unsafe { LoadIconW(Some(instance.into()), APPLICATION_ICON) }
}

pub(crate) fn load_tray_icon() -> windows::core::Result<Icon> {
    let instance = unsafe { GetModuleHandleW(None)? };
    let icon = unsafe { LoadIconMetric(Some(instance.into()), APPLICATION_ICON, LIM_SMALL)? };
    // LoadIconMetric returns an owned icon. The tray will destroy it on drop.
    Ok(Icon::from_handle(icon.0 as isize))
}

pub(crate) fn apply_application_icon(window: HWND) -> windows::core::Result<()> {
    let icon = load_application_icon()?;
    unsafe {
        SendMessageW(
            window,
            WM_SETICON,
            Some(WPARAM(ICON_BIG as usize)),
            Some(LPARAM(icon.0 as isize)),
        );
        SendMessageW(
            window,
            WM_SETICON,
            Some(WPARAM(ICON_SMALL as usize)),
            Some(LPARAM(icon.0 as isize)),
        );
    }
    Ok(())
}
