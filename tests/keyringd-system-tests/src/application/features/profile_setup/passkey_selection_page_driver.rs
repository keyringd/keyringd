use anyhow::Result;

pub trait PasskeySelectionPageDriver {
    fn candidate_labels(&self) -> Result<Vec<String>>;
    fn select_passkey(&self, label: &str) -> Result<()>;
    fn continue_recovery(&self) -> Result<()>;
}
