#[derive(Debug)]
pub struct ProfileSetupPinContent {
    pub texts: Vec<String>,
    pub pin_enabled: bool,
    pub confirmation_enabled: Option<bool>,
    pub submit_visible: bool,
}
