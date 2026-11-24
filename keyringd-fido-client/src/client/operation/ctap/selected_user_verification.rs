use crate::PinUvAuthProtocol;

/// User verification method selected from this request and the ability of the authenticator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedUserVerification {
    None,
    BuiltInUvOption,
    BuiltInUvToken(PinUvAuthProtocol),
    ClientPin(PinUvAuthProtocol),
    ClientPinSetup {
        protocol: PinUvAuthProtocol,
        minimum_length: u64,
    },
}
