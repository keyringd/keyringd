use keyringd_core::application::PasskeyCredentialCandidate;
use keyringd_core::model::passkey::CredentialId;

#[derive(Clone, PartialEq)]
pub struct PasskeyCredentialSelection {
    candidates: Vec<PasskeyCredentialCandidate>,
    selected_index: usize,
}

impl PasskeyCredentialSelection {
    pub fn new(candidates: Vec<PasskeyCredentialCandidate>) -> Self {
        Self {
            candidates,
            selected_index: 0,
        }
    }

    pub fn candidates(&self) -> &[PasskeyCredentialCandidate] {
        &self.candidates
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn select(&mut self, index: usize) {
        if index < self.candidates.len() {
            self.selected_index = index;
        }
    }

    pub fn selected_id(&self) -> Option<CredentialId> {
        self.candidates
            .get(self.selected_index)
            .map(|candidate| candidate.credential_id().clone())
    }
}
