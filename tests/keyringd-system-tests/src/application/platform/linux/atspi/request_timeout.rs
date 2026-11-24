use std::time::Duration;

pub(in crate::application::platform::linux) const ATSPI_RESPONSE_TIMEOUT: Duration =
    Duration::from_secs(2);
