use windows::Win32::Foundation::HWND;

pub trait WebAuthnWindow: Send + Sync {
    type Session;

    fn handle(&self) -> HWND;
    fn activate(&self) -> Self::Session;
}
