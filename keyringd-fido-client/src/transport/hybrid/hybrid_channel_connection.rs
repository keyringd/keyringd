use crate::HybridTransportUnavailableReason;

/// Results of a Hybrid connection. Returns a connected CTAP channel or a reason why Hybrid is not available.
pub(crate) enum HybridChannelConnection {
    Connected(Box<dyn crate::CtapTransport>),
    Unavailable(HybridTransportUnavailableReason),
}
