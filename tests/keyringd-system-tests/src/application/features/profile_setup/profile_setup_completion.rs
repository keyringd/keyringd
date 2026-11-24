#[derive(Debug)]
pub struct ProfileSetupCompletion {
    pub messages: Vec<String>,
    pub public_keys_enabled: bool,
    pub done_enabled: bool,
}
