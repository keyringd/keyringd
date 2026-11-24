use std::process::ExitStatus;

pub(super) enum ProcessOutcome {
    Exited(ExitStatus),
    TimedOut,
}
