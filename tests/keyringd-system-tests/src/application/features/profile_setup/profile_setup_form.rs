#[derive(Debug)]
pub struct ProfileSetupForm {
    pub name: String,
    pub existing_passkey: bool,
    pub new_passkey: bool,
    pub create_enabled: bool,
    pub create_label: String,
    pub messages: Vec<String>,
}
