use windows::Win32::Foundation::HWND;

pub(super) struct OwnedWindowLookup {
    pub(super) owner: usize,
    pub(super) windows: Vec<HWND>,
}
