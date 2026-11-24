use std::path::Path;

use crate::app::ui::util::place_task_dialog_on_cursor_monitor;
use keyringd_core::application::ApplicationError;
use windows::Win32::UI::Controls::{
    TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TASKDIALOGCONFIG_0, TD_ERROR_ICON, TDCBF_CLOSE_BUTTON,
    TDF_ALLOW_DIALOG_CANCELLATION, TaskDialogIndirect,
};
use windows::core::{PCWSTR, w};

pub(super) fn show(error: &anyhow::Error, data_dir: Option<&Path>) -> anyhow::Result<()> {
    let detail = message(error)
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let logs = data_dir
        .map(|directory| directory.join("logs"))
        .filter(|path| path.is_dir());
    let open_logs = TASKDIALOG_BUTTON {
        nButtonID: 1001,
        pszButtonText: w!("Open logs"),
    };
    // This dialog must work before the prompt window, icon, logging or UI runtime exists.
    let config = TASKDIALOGCONFIG {
        cbSize: std::mem::size_of::<TASKDIALOGCONFIG>() as u32,
        dwFlags: TDF_ALLOW_DIALOG_CANCELLATION,
        dwCommonButtons: TDCBF_CLOSE_BUTTON,
        pszWindowTitle: w!("Keyringd"),
        pszMainInstruction: w!("Keyringd could not start"),
        pszContent: PCWSTR(detail.as_ptr()),
        cButtons: u32::from(logs.is_some()),
        pButtons: if logs.is_some() {
            &open_logs
        } else {
            std::ptr::null()
        },
        Anonymous1: TASKDIALOGCONFIG_0 {
            pszMainIcon: TD_ERROR_ICON,
        },
        pfCallback: Some(place_task_dialog_on_cursor_monitor),
        ..Default::default()
    };
    let mut selected = 0;
    unsafe { TaskDialogIndirect(&config, Some(&mut selected), None, None)? };
    if selected == open_logs.nButtonID
        && let Some(logs) = logs
    {
        std::process::Command::new("explorer.exe")
            .arg(logs)
            .spawn()?;
    }
    Ok(())
}

fn message(error: &anyhow::Error) -> String {
    let explanation = if let Some(application_error) = error.downcast_ref::<ApplicationError>() {
        let failure = super::ApplicationLoadFailure::from_error(application_error);
        tracing::error!(
            category = failure.log_category(),
            "application initialization failed"
        );
        failure.user_message()
    } else {
        "Keyringd could not start."
    };
    format!("{explanation}\n\nDetails:\n{error:#}")
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "message" {
            case "Profile failure keeps its explanation and error context" {
                verify "retain both the profile-specific explanation and the underlying error" {
                    let error = anyhow::Error::new(ApplicationError::ProfileRepository(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "invalid JSON at line 3",
                    )))
                    .context("loading profiles.json");
                    let detail = message(&error);
                    assert!(detail.contains("The saved profile data is invalid"));
                    assert!(detail.contains("loading profiles.json"));
                    assert!(detail.contains("invalid JSON at line 3"));
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "message" {
            case "Early startup failure includes the cause without a profile error" {
                verify "include the original startup error when profile loading has not begun" {
                    let error = anyhow::Error::new(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "access denied",
                    ))
                    .context("creating the log directory");
                    let detail = message(&error);
                    assert!(detail.contains("Keyringd could not start"));
                    assert!(detail.contains("creating the log directory: access denied"));
                }
            }
        }
    }
}
