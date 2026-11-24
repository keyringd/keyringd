use windows::Win32::Foundation::HWND;

pub(super) struct WindowLookup {
    pub(super) pid: u32,
    pub(super) windows: Vec<HWND>,
    pub(super) visible_only: bool,
}
