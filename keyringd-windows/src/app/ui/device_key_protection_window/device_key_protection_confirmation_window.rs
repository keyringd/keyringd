use keyringd_core::service::DeviceKeyProtectionChange;

use keyringd_ui_core::text as ui_text;

use windows::Win32::Foundation::HWND;

use windows::Win32::UI::Controls::{
    TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TASKDIALOGCONFIG_0, TDCBF_CANCEL_BUTTON,
    TDF_ALLOW_DIALOG_CANCELLATION, TDF_POSITION_RELATIVE_TO_WINDOW, TDF_USE_COMMAND_LINKS,
    TDF_USE_HICON_MAIN, TaskDialogIndirect,
};
use windows::core::{PCWSTR, w};

use crate::app::ui::util::{load_application_icon, place_task_dialog_on_cursor_monitor};

pub(crate) struct DeviceKeyProtectionConfirmationWindow {
    owner: usize,
}

impl DeviceKeyProtectionConfirmationWindow {
    pub(crate) fn new(owner: usize) -> Self {
        Self { owner }
    }

    pub(crate) fn confirm(
        &self,
        profile_name: &str,
        action: DeviceKeyProtectionChange,
    ) -> windows::core::Result<bool> {
        let explanation = match action {
            DeviceKeyProtectionChange::Enable => {
                "This creates a replacement Device Key. Windows Security will configure verification for its use, then your Passkey will authorize it."
            }
            DeviceKeyProtectionChange::ChangePassword => {
                "This creates a replacement Device Key with new local authentication settings, then your Passkey will authorize it."
            }
            DeviceKeyProtectionChange::Disable => {
                "This creates a replacement Device Key that SSH can use in the background, then your Passkey will authorize it."
            }
        };
        self.show(
            &keyringd_ui_core::device_key_protection_window::heading(profile_name, action),
            &keyringd_ui_core::device_key_protection_window::confirmation(explanation),
            ui_text::CONTINUE,
        )
    }

    pub(super) fn confirm_retry(&self, error: &str) -> windows::core::Result<bool> {
        self.show(
            ui_text::DEVICE_KEY_PROTECTION_ERROR_TITLE,
            error,
            ui_text::DEVICE_KEY_PROTECTION_RETRY,
        )
    }

    fn show(&self, heading: &str, detail: &str, action: &str) -> windows::core::Result<bool> {
        let instruction = Self::wide_string(heading);
        let content = Self::wide_string(detail);
        let continue_label = Self::wide_string(action);
        let button = TASKDIALOG_BUTTON {
            nButtonID: 1001,
            pszButtonText: PCWSTR(continue_label.as_ptr()),
        };
        let icon = load_application_icon()?;
        let config = TASKDIALOGCONFIG {
            cbSize: std::mem::size_of::<TASKDIALOGCONFIG>() as u32,
            hwndParent: HWND(self.owner as *mut core::ffi::c_void),
            dwFlags: TDF_ALLOW_DIALOG_CANCELLATION
                | TDF_POSITION_RELATIVE_TO_WINDOW
                | TDF_USE_COMMAND_LINKS
                | TDF_USE_HICON_MAIN,
            dwCommonButtons: TDCBF_CANCEL_BUTTON,
            pszWindowTitle: w!("Keyringd"),
            pszMainInstruction: PCWSTR(instruction.as_ptr()),
            pszContent: PCWSTR(content.as_ptr()),
            cButtons: 1,
            pButtons: &button,
            nDefaultButton: button.nButtonID,
            pfCallback: Some(place_task_dialog_on_cursor_monitor),
            Anonymous1: TASKDIALOGCONFIG_0 { hMainIcon: icon },
            ..Default::default()
        };
        let mut selected = 0;
        unsafe { TaskDialogIndirect(&config, Some(&mut selected), None, None)? };
        Ok(selected == button.nButtonID)
    }

    fn wide_string(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}
