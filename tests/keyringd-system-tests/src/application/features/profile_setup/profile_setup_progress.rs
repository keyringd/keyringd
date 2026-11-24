/// A snapshot of the actual interaction page, including controls absent during transitions.
#[derive(Debug, Default)]
pub struct ProfileSetupProgress {
    pub heading: String,
    pub texts: Vec<String>,
    pub security_key_enabled: Option<bool>,
    pub pin_visible: bool,
    pub continue_enabled: Option<bool>,
}
