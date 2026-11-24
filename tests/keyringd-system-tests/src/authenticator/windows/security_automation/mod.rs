//! The elevated broker only automates Windows Security dialogs. One runner
//! owns it and shares its authenticated endpoint with its scenario processes.
mod security_automation_action;
mod security_automation_client;
mod security_automation_request;
mod server;
pub use security_automation_action::SecurityAutomationAction;
pub use security_automation_client::SecurityAutomationClient;
pub use server::serve;
