//! Shared wire contract for the private macOS UI fixture (no AppKit dependency).
mod control_command;
pub use control_command::ControlCommand;

mod selector;
pub use selector::Selector;

mod scope;
pub use scope::Scope;

mod target;
pub use target::Target;

mod ui_command;
pub use ui_command::UiCommand;

mod element;
pub use element::Element;

mod ui_response;
pub use ui_response::UiResponse;

mod fixture_state;
pub use fixture_state::FixtureState;

mod control_response;
pub use control_response::ControlResponse;
