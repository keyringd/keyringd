use keyringd_ui_core::ssh_client_configuration_window::SshClientConfigurationCopyTarget;
use keyringd_ui_core::text as ui_text;
use windows::Win32::Foundation::HWND;

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::Controls::{
    TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TASKDIALOGCONFIG_0, TDCBF_CANCEL_BUTTON,
    TDCBF_CLOSE_BUTTON, TDCBF_OK_BUTTON, TDF_ALLOW_DIALOG_CANCELLATION,
    TDF_POSITION_RELATIVE_TO_WINDOW, TDF_USE_COMMAND_LINKS, TDF_USE_HICON_MAIN, TDN_BUTTON_CLICKED,
    TaskDialogIndirect,
};
use windows::core::{HRESULT, HSTRING, PCWSTR, w};

use crate::app::ui::ssh_client_configuration_window::SshClientConfigurationCallbackContext;
use crate::app::ui::ssh_client_configuration_window::SshClientConfigurationEvent;
use crate::app::ui::util::{load_application_icon, place_task_dialog_on_cursor_monitor};

const CONFIGURE_WINDOWS_BUTTON_ID: i32 = 1006;
const COPY_WSL_SETTING_BUTTON_ID: i32 = 1007;
const COPY_GIT_FOR_WINDOWS_SETTING_BUTTON_ID: i32 = 1008;

pub(super) struct SshClientConfigurationNativeWindow {
    owner: usize,
}

impl SshClientConfigurationNativeWindow {
    pub(super) fn new(owner: usize) -> Self {
        Self { owner }
    }

    pub(super) fn show(
        &self,
        state: &keyringd_ui_core::ssh_client_configuration_window::SshClientConfigurationState,
        handler: &mut dyn FnMut(SshClientConfigurationEvent),
    ) -> windows::core::Result<()> {
        let configured = state.configured();
        let title = HSTRING::from(ui_text::SSH_CLIENT_CONFIGURATION_WINDOW_TITLE);
        let heading = HSTRING::from(ui_text::SSH_CLIENT_CONFIGURATION_HEADING);
        let buttons = [
            TASKDIALOG_BUTTON {
                nButtonID: CONFIGURE_WINDOWS_BUTTON_ID,
                pszButtonText: if configured {
                    w!("SSH_AUTH_SOCK: Unconfigure\nRemove the Keyringd user environment setting.")
                } else {
                    w!("SSH_AUTH_SOCK: Configure\nUse Keyringd as your SSH agent.")
                },
            },
            TASKDIALOG_BUTTON {
                nButtonID: COPY_GIT_FOR_WINDOWS_SETTING_BUTTON_ID,
                pszButtonText: w!(
                    "Copy Git for Windows SSH setting\nCopy the environment setting to add to your Git Bash shell configuration."
                ),
            },
            TASKDIALOG_BUTTON {
                nButtonID: COPY_WSL_SETTING_BUTTON_ID,
                pszButtonText: w!(
                    "Copy WSL SSH setting\nRequires the separate keyringd-wsl .deb or .rpm package inside WSL."
                ),
            },
        ];
        let content = HSTRING::from(format!(
            "Configure Windows OpenSSH, Git for Windows, or WSL to use the Keyringd agent.\n\nServer Public Keys belong to individual profiles and can be copied from the profile menu.{}",
            state
                .status_message()
                .map(|status| format!("\n\n{status}"))
                .unwrap_or_default()
        ));
        let mut context = SshClientConfigurationCallbackContext { handler };
        self.show_dialog(
            TASKDIALOGCONFIG {
                dwFlags: TDF_ALLOW_DIALOG_CANCELLATION | TDF_USE_COMMAND_LINKS,
                dwCommonButtons: TDCBF_CLOSE_BUTTON,
                pszWindowTitle: PCWSTR(title.as_ptr()),
                pszMainInstruction: PCWSTR(heading.as_ptr()),
                pszContent: PCWSTR(content.as_ptr()),
                cButtons: buttons.len() as u32,
                pButtons: buttons.as_ptr(),
                nDefaultButton: CONFIGURE_WINDOWS_BUTTON_ID,
                pfCallback: Some(Self::callback),
                lpCallbackData: std::ptr::from_mut(&mut context) as isize,
                ..Default::default()
            },
            None,
        )
    }

    pub(super) fn confirm_windows_ssh_setup(&self, removing: bool) -> windows::core::Result<bool> {
        self.show_continue(
            "SSH_AUTH_SOCK", keyringd_ui_core::ssh_client_configuration_window::confirmation_heading(removing),
            if removing { "This removes SSH_AUTH_SOCK if it still points to Keyringd. Restart your applications and terminals to apply the change." } else { "This sets the SSH_AUTH_SOCK user environment variable. Restart your applications and terminals to apply the change." }, if removing { ui_text::SSH_CLIENT_CONFIGURATION_UNCONFIGURE } else { ui_text::SSH_CLIENT_CONFIGURATION_CONFIGURE },
        )
    }

    pub(super) fn show_windows_ssh_configured(&self, removing: bool) -> windows::core::Result<()> {
        self.show_message(
            if removing { "SSH_AUTH_SOCK unconfigured" } else { "SSH_AUTH_SOCK configured" }, "Restart your applications to apply the setting",
            "Close and reopen any terminals you are using so Windows SSH picks up the new agent setting.",
        )
    }

    pub(super) fn confirm_wsl_ssh_setting_copy(&self) -> windows::core::Result<bool> {
        self.show_continue(
            "Use Keyringd from WSL", "Copy the WSL OpenSSH agent setting",
            "The Windows app alone does not enable WSL support. First install the separate keyringd-wsl .deb or .rpm package inside each WSL distribution where you want to use it. Then add this line to your shell startup file inside WSL. This button only copies the setting; it does not change your configuration:\n\nexport SSH_AUTH_SOCK=\"$XDG_RUNTIME_DIR/keyringd-agent.sock\"", "Copy SSH setting",
        )
    }

    pub(super) fn confirm_git_for_windows_ssh_setting_copy(&self) -> windows::core::Result<bool> {
        let content = format!(
            "Add this line to ~/.bash_profile, then open a new Git Bash terminal. This button only copies the setting; it does not change your configuration:\n\n{}\n\nKeep Keyringd running when using SSH or Git.",
            crate::system::windows_ssh_client_configuration::GIT_FOR_WINDOWS_SSH_SETTING,
        );
        self.show_continue(
            "Use Keyringd with Git for Windows",
            "Copy the Git Bash SSH agent setting",
            &content,
            "Copy SSH setting",
        )
    }

    fn show_continue(
        &self,
        title: &str,
        instruction: &str,
        content: &str,
        button_text: &str,
    ) -> windows::core::Result<bool> {
        let title = HSTRING::from(title);
        let instruction = HSTRING::from(instruction);
        let content = HSTRING::from(content);
        let button_text = HSTRING::from(button_text);
        let button = TASKDIALOG_BUTTON {
            nButtonID: 1001,
            pszButtonText: PCWSTR(button_text.as_ptr()),
        };
        let mut selected = 0;
        self.show_dialog(
            TASKDIALOGCONFIG {
                dwFlags: TDF_ALLOW_DIALOG_CANCELLATION | TDF_USE_COMMAND_LINKS,
                dwCommonButtons: TDCBF_CANCEL_BUTTON,
                pszWindowTitle: PCWSTR(title.as_ptr()),
                pszMainInstruction: PCWSTR(instruction.as_ptr()),
                pszContent: PCWSTR(content.as_ptr()),
                cButtons: 1,
                pButtons: &button,
                nDefaultButton: button.nButtonID,
                ..Default::default()
            },
            Some(&mut selected),
        )?;
        Ok(selected == button.nButtonID)
    }

    fn show_message(
        &self,
        title: &str,
        instruction: &str,
        content: &str,
    ) -> windows::core::Result<()> {
        let title = HSTRING::from(title);
        let instruction = HSTRING::from(instruction);
        let content = HSTRING::from(content);
        self.show_dialog(
            TASKDIALOGCONFIG {
                dwCommonButtons: TDCBF_OK_BUTTON,
                pszWindowTitle: PCWSTR(title.as_ptr()),
                pszMainInstruction: PCWSTR(instruction.as_ptr()),
                pszContent: PCWSTR(content.as_ptr()),
                ..Default::default()
            },
            None,
        )
    }

    fn show_dialog(
        &self,
        mut config: TASKDIALOGCONFIG,
        selected: Option<&mut i32>,
    ) -> windows::core::Result<()> {
        let icon = load_application_icon()?;
        config.cbSize = std::mem::size_of::<TASKDIALOGCONFIG>() as u32;
        config.hwndParent = HWND(self.owner as *mut core::ffi::c_void);
        config.dwFlags = config.dwFlags | TDF_POSITION_RELATIVE_TO_WINDOW | TDF_USE_HICON_MAIN;
        config.Anonymous1 = TASKDIALOGCONFIG_0 { hMainIcon: icon };
        config.pfCallback = config
            .pfCallback
            .or(Some(place_task_dialog_on_cursor_monitor));
        unsafe { TaskDialogIndirect(&config, selected.map(std::ptr::from_mut), None, None) }
    }

    unsafe extern "system" fn callback(
        window: HWND,
        notification: windows::Win32::UI::Controls::TASKDIALOG_NOTIFICATIONS,
        button: WPARAM,
        lparam: LPARAM,
        callback_data: isize,
    ) -> HRESULT {
        if notification != TDN_BUTTON_CLICKED {
            return place_task_dialog_on_cursor_monitor(window, notification, button, lparam, 0);
        }
        let event = match button.0 as i32 {
            CONFIGURE_WINDOWS_BUTTON_ID => SshClientConfigurationEvent::ConfigureRequested,
            COPY_WSL_SETTING_BUTTON_ID => {
                SshClientConfigurationEvent::CopyRequested(SshClientConfigurationCopyTarget::Wsl)
            }
            COPY_GIT_FOR_WINDOWS_SETTING_BUTTON_ID => SshClientConfigurationEvent::CopyRequested(
                SshClientConfigurationCopyTarget::GitForWindows,
            ),
            _ => return HRESULT(0),
        };
        let context =
            unsafe { &mut *(callback_data as *mut SshClientConfigurationCallbackContext<'_>) };
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (context.handler)(event)))
            .is_err()
        {
            tracing::error!("getting-started event handler panicked");
        }
        HRESULT(0)
    }
}
