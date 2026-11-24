#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SecurityKeyPinPrompt {
    pub reason: String,
    pub attempts_left: Option<u32>,
    pub minimum_length: Option<u32>,
    pub new_pin: bool,
}
