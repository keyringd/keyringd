mod profile_details_command;
mod profile_details_event;
mod profile_details_state;

pub use profile_details_command::ProfileDetailsCommand;
pub use profile_details_event::ProfileDetailsEvent;
pub use profile_details_state::ProfileDetailsState;

mod labels;
pub use labels::{heading, replacement_description, status};
