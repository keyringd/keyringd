#[derive(Clone, Copy, Debug)]
pub enum ProfileSetupStep {
    CreatePasskey,
    FindPasskey,
    RecoverPasskeyPublicKey,
    AuthorizeNewDevice,
    AuthorizeRecoveredDevice,
    Completed,
}

impl ProfileSetupStep {
    pub fn title(self) -> &'static str {
        match self {
            Self::CreatePasskey => "Step 1/2 · Create Passkey",
            Self::FindPasskey => "Step 1/3 · Find Passkey",
            Self::RecoverPasskeyPublicKey => "Step 2/3 · Verify Passkey",
            Self::AuthorizeNewDevice => "Step 2/2 · Authorize this computer",
            Self::AuthorizeRecoveredDevice => "Step 3/3 · Authorize this computer",
            Self::Completed => "SSH Profile Created",
        }
    }
}
