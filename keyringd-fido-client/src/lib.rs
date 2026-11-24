extern crate self as keyringd_fido_client;
mod client;
mod ctap;
mod transport;
mod util;

pub use client::backend;
pub use client::{
    AssertionResponse, ClientData, ClientDataProvider, ClientDataProviderError, DEFAULT_TIMEOUT,
    FidoClient, FidoClientError, FidoInteractionHandler, FidoInteractionHandlerError,
    GetAssertionResult, GetAssertionsContext, GetAssertionsResult, InteractionId,
    InteractionResponse, InternalAuthenticator, MakeCredentialContext, MakeCredentialResult,
    NewPinInteraction, OperationId, OperationStatus, PinInteraction, ResponseValidationError,
    TransportInteraction,
};
pub(crate) use client::{
    GetAssertionOperationContext, MakeCredentialOperationContext, TransportSelection,
};
pub(crate) use ctap::{
    AuthenticatorCapabilities, AuthenticatorCapabilityError, AuthenticatorRequest,
    AuthenticatorRequirements, CtapClient, CtapClientError, PinUvAuthCryptoError, PinUvAuthError,
    PinUvAuthProtocol, PinUvAuthToken,
};
#[cfg(feature = "test-support")]
pub use transport::hybrid::HybridTunnelTestRoute;
pub use transport::usb;
pub(crate) use transport::{
    CtapTransport, CtapTransportError, CtapTransportStatus, HybridAdvertisement, HybridBleError,
    HybridBleScanner, HybridBleScannerAvailability, HybridChannelConnection,
    HybridChannelConnector, HybridConnectionError, HybridCryptoError, HybridCtapTransport,
    HybridCtapTransportError, HybridDataChannel, HybridNoiseError, HybridNoiseSession,
    HybridOperationHint, HybridPlaintextMessage, HybridQrInitiatedTransaction, HybridSecret64,
    HybridTunnelEndpoint, HybridWebSocketError, SystemHybridChannelConnector, TransportError,
    WebSocketHybridDataChannel,
};
pub use transport::{HybridAdvertisementRelay, HybridTransportUnavailableReason};

pub(crate) use client::SelectedUserVerification;

#[cfg(test)]
mod test_support;

#[cfg(test)]
mod tests;
