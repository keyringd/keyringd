use super::ProfileSetupProgress;

/// Receives profile setup stage notifications. Cancellation is handled separately.
pub trait ProfileSetupProgressReporter: Send {
    fn report_progress(&mut self, progress: ProfileSetupProgress);
}

impl<F: FnMut(ProfileSetupProgress) + Send> ProfileSetupProgressReporter for F {
    fn report_progress(&mut self, progress: ProfileSetupProgress) {
        self(progress);
    }
}
