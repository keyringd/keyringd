/// Passwords collected for enabling, changing, or disabling Device Key protection.
pub enum DeviceKeyPasswordChangeInput {
    Enable {
        new_password: String,
    },
    Change {
        current_password: String,
        new_password: String,
    },
    Disable {
        current_password: String,
    },
}
