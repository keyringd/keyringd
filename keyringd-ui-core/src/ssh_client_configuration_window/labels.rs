use crate::text::{
    SSH_CLIENT_CONFIGURATION_CONFIGURE_QUESTION as CONFIGURE_QUESTION,
    SSH_CLIENT_CONFIGURATION_UNCONFIGURE_QUESTION as UNCONFIGURE_QUESTION,
};

pub fn confirmation_heading(removing: bool) -> &'static str {
    if removing {
        UNCONFIGURE_QUESTION
    } else {
        CONFIGURE_QUESTION
    }
}
