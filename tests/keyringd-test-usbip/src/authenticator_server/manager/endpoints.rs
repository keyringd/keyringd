use super::io;
use crate::authenticator_server::manager::{
    AuthenticatorServerOptions, AuthenticatorServerProcessError,
};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener};

pub(crate) struct Endpoints {
    pub usb: SocketAddr,
    pub control: SocketAddr,
    pub hybrid: Option<SocketAddr>,
}
impl Endpoints {
    pub fn reserve(
        options: &AuthenticatorServerOptions,
    ) -> Result<(Self, Vec<TcpListener>), AuthenticatorServerProcessError> {
        let mut listeners = Vec::new();
        let mut reserve = |address| -> Result<SocketAddr, AuthenticatorServerProcessError> {
            let listener = TcpListener::bind(address).map_err(|e| io("reserve listener", e))?;
            let address = listener
                .local_addr()
                .map_err(|e| io("read reserved listener address", e))?;
            listeners.push(listener);
            Ok(address)
        };
        // Keep all reservations until all addresses are chosen.
        let usb = reserve(options.usb_address)?;
        let control = reserve(options.control_address)?;
        let hybrid = options
            .hybrid
            .as_ref()
            .map(|h| reserve(h.address))
            .transpose()?;
        Ok((
            Self {
                usb,
                control,
                hybrid,
            },
            listeners,
        ))
    }
}
/// An unspecified bind address is connected to through the local loopback.
pub(crate) fn connect_address(mut address: SocketAddr) -> SocketAddr {
    if address.ip().is_unspecified() {
        address.set_ip(match address.ip() {
            IpAddr::V4(_) => Ipv4Addr::LOCALHOST.into(),
            IpAddr::V6(_) => Ipv6Addr::LOCALHOST.into(),
        });
    }
    address
}
