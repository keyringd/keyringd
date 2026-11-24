use windows::Win32::Foundation::HWND;

pub(super) struct NamedWindowLookup {
    pub(super) title: String,
    pub(super) windows: Vec<HWND>,
}
