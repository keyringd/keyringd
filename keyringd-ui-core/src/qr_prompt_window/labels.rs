use crate::text;
use crate::text::QR_PROMPT_BLUETOOTH_REQUIRED as BLUETOOTH_REQUIRED;
use keyringd_fido_client::HybridTransportUnavailableReason;

pub fn hybrid_unavailable_message(
    reason: Option<HybridTransportUnavailableReason>,
) -> &'static str {
    match reason {
        Some(HybridTransportUnavailableReason::NoBluetoothAdapter) => text::QR_PROMPT_NO_BLUETOOTH,
        Some(HybridTransportUnavailableReason::BluetoothDisabled) => {
            text::QR_PROMPT_BLUETOOTH_DISABLED
        }
        None => BLUETOOTH_REQUIRED,
    }
}
