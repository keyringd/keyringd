use crate::application::ApplicationError;
use keyringd_fido::{AuthenticatorTransport, webauthn::PublicKeyCredentialHint};
use keyringd_fido_client::usb::UsbHidDeviceId;

/// Retains the first successful route across an execution's FIDO requests.
/// Hybrid retains the route, not a connection to a particular phone.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum FidoOperationTarget {
    Unselected,
    Usb { device_id: Option<UsbHidDeviceId> },
    Hybrid,
}

impl FidoOperationTarget {
    pub(crate) fn hints(&self) -> Vec<PublicKeyCredentialHint> {
        match self {
            Self::Unselected => Vec::new(),
            Self::Usb { .. } => vec![PublicKeyCredentialHint::security_key()],
            Self::Hybrid => vec![PublicKeyCredentialHint::hybrid()],
        }
    }

    pub(crate) fn usb_device_id(&self) -> Option<&UsbHidDeviceId> {
        match self {
            Self::Usb { device_id } => device_id.as_ref(),
            Self::Unselected | Self::Hybrid => None,
        }
    }

    pub(crate) fn remember(
        &mut self,
        transport: &AuthenticatorTransport,
        usb_device_id: Option<&UsbHidDeviceId>,
    ) -> Result<(), ApplicationError> {
        let selected = if transport == &AuthenticatorTransport::usb() {
            Self::Usb {
                device_id: usb_device_id.cloned(),
            }
        } else if transport == &AuthenticatorTransport::hybrid() && usb_device_id.is_none() {
            Self::Hybrid
        } else {
            return Err(ApplicationError::InvalidAuthenticatorResponse(
                "Passkey result must use USB, or Hybrid without a USB device ID".into(),
            ));
        };
        match (self, selected) {
            (target @ Self::Unselected, selected) => *target = selected,
            (Self::Hybrid, Self::Hybrid) => {}
            (
                Self::Usb { device_id },
                Self::Usb {
                    device_id: selected_id,
                },
            ) => match (device_id.as_ref(), selected_id) {
                (Some(current), Some(selected)) if current != &selected => {
                    return Err(ApplicationError::InvalidAuthenticatorResponse(
                        "Passkey result changed the selected USB device".into(),
                    ));
                }
                (None, Some(selected)) => *device_id = Some(selected),
                _ => {}
            },
            _ => {
                return Err(ApplicationError::InvalidAuthenticatorResponse(
                    "Passkey result changed the selected transport".into(),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_hints_and_learns_a_usb_device_without_losing_it() {
        let mut target = FidoOperationTarget::Unselected;
        assert_eq!(target, FidoOperationTarget::Unselected);
        assert!(target.hints().is_empty());
        assert!(target.usb_device_id().is_none());
        target
            .remember(&AuthenticatorTransport::usb(), None)
            .unwrap();
        assert_eq!(target.hints(), [PublicKeyCredentialHint::security_key()]);
        assert!(target.usb_device_id().is_none());
        let device = UsbHidDeviceId::new(b"selected-key".to_vec());
        target
            .remember(&AuthenticatorTransport::usb(), Some(&device))
            .unwrap();
        target
            .remember(&AuthenticatorTransport::usb(), None)
            .unwrap();
        target
            .remember(&AuthenticatorTransport::usb(), Some(&device))
            .unwrap();
        assert_eq!(target.usb_device_id(), Some(&device));
        assert_eq!(target.hints(), [PublicKeyCredentialHint::security_key()]);
        let other = UsbHidDeviceId::new(b"other-key".to_vec());
        assert!(
            target
                .remember(&AuthenticatorTransport::usb(), Some(&other))
                .is_err()
        );
        assert_eq!(target.usb_device_id(), Some(&device));
        assert!(
            target
                .remember(&AuthenticatorTransport::hybrid(), None)
                .is_err()
        );
        assert_eq!(target.usb_device_id(), Some(&device));
    }

    #[test]
    fn hybrid_remains_hybrid_and_never_gains_a_usb_device() {
        let mut target = FidoOperationTarget::Unselected;
        target
            .remember(&AuthenticatorTransport::hybrid(), None)
            .unwrap();
        target
            .remember(&AuthenticatorTransport::hybrid(), None)
            .unwrap();
        assert_eq!(target.hints(), [PublicKeyCredentialHint::hybrid()]);
        let device = UsbHidDeviceId::new(b"usb-key".to_vec());
        assert!(
            target
                .remember(&AuthenticatorTransport::usb(), Some(&device))
                .is_err()
        );
        assert!(
            target
                .remember(&AuthenticatorTransport::hybrid(), Some(&device))
                .is_err()
        );
        assert_eq!(target, FidoOperationTarget::Hybrid);
        assert!(target.usb_device_id().is_none());
    }

    #[test]
    fn rejects_unsupported_or_inconsistent_results_without_selecting_a_target() {
        let device = UsbHidDeviceId::new(b"usb-key".to_vec());
        let mut target = FidoOperationTarget::Unselected;
        for transport in [
            AuthenticatorTransport::internal(),
            AuthenticatorTransport::nfc(),
            AuthenticatorTransport::new("future"),
        ] {
            assert!(target.remember(&transport, None).is_err());
            assert_eq!(target, FidoOperationTarget::Unselected);
        }
        assert!(
            target
                .remember(&AuthenticatorTransport::hybrid(), Some(&device))
                .is_err()
        );
        assert_eq!(target, FidoOperationTarget::Unselected);
    }
}
