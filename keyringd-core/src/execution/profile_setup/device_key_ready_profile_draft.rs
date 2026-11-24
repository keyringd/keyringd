use crate::model::{Profile, ProfileId, device_key::DeviceKey, passkey::Passkey};

pub(super) struct DeviceKeyReadyProfileDraft {
    id: ProfileId,
    display_name: String,
    passkey: Passkey,
    device_key: DeviceKey,
}

impl DeviceKeyReadyProfileDraft {
    pub(super) fn new(
        id: ProfileId,
        display_name: String,
        passkey: Passkey,
        device_key: DeviceKey,
    ) -> Self {
        Self {
            id,
            display_name,
            passkey,
            device_key,
        }
    }

    pub(super) fn complete(self) -> Result<Profile, &'static str> {
        Profile::new(
            self.id,
            self.display_name,
            true,
            self.device_key,
            self.passkey,
        )
    }
}
