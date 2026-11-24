use super::{PasskeyCredentialSelection, TransportPrompts};
use crate::pin_prompt_window::SecurityKeyPinPrompt;
use keyringd_fido_client::InteractionId;

/// Content shown while setup runs. Replacing it also discards the previous input's routing ID.
#[derive(Clone, PartialEq, Default)]
pub enum ProfileSetupPrompt {
    #[default]
    Hidden,
    Transports(TransportPrompts),
    Pin {
        request_id: InteractionId,
        request: SecurityKeyPinPrompt,
    },
    Preparing,
    CredentialSelection(PasskeyCredentialSelection),
    Cancelling {
        close_window: bool,
    },
}
