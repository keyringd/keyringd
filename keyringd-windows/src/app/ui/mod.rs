pub(crate) mod about_window;
mod component_model;
pub(crate) mod toast;
pub(crate) mod tray;
pub(crate) mod util;

pub(crate) use component_model::{ComponentModel, reduce};
mod dialog_layout;
use dialog_layout::{PASSKEY_CONTENT_WIDTH, PASSKEY_DIALOG_WIDTH};
pub(crate) mod agent_prompt_window;
mod auxiliary_window;
pub(crate) mod device_key_protection_window;
pub(crate) mod pin_prompt_window;
pub(crate) mod profile_details_window;
pub(crate) mod profile_list;
pub(crate) mod profile_list_item;
pub(crate) mod profile_setup_interaction;
pub(crate) mod profile_setup_window;
pub(crate) mod profile_setup_wizard;
pub(crate) mod profiles_window;
pub(crate) mod qr_prompt_window;
pub(crate) mod remove_profile_window;
pub(crate) mod replace_device_key_window;
pub(crate) mod ssh_client_configuration_window;
pub(crate) mod ssh_connection_test_window;
pub(crate) mod tray_menu;
pub(crate) mod trust_entries_window;
pub(in crate::app::ui) use auxiliary_window::AuxiliaryWindow;
mod prompt_window_error;
pub(crate) use prompt_window_error::PromptWindowError;
#[cfg(test)]
mod test_support;

pub(crate) mod third_party_licenses_window;
