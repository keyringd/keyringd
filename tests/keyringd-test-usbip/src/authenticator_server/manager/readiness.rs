use super::{Endpoints, OwnedProcess, connect_address};
use crate::authenticator_server::{
    client::AuthenticatorServerClient, manager::AuthenticatorServerProcessError,
};
use std::{net::SocketAddr, time::Duration};
use tokio::{
    net::TcpStream,
    time::{Instant, sleep, timeout},
};

pub(crate) async fn wait_ready(
    process: &mut OwnedProcess,
    endpoints: &Endpoints,
    deadline_after: Duration,
) -> Result<(), AuthenticatorServerProcessError> {
    let deadline = Instant::now() + deadline_after;
    let client = AuthenticatorServerClient::new(connect_address(endpoints.control))
        .with_timeout(Duration::from_millis(500));
    let check = async {
        let mut last_error;
        loop {
            if let Some(status) = process.status()? {
                return Err(AuthenticatorServerProcessError::ExitedEarly {
                    status,
                    log_tail: process.log_tail(),
                });
            }
            match client.health().await {
                Err(error) => last_error = format!("control health: {error}"),
                Ok(()) => {
                    let mut ready = true;
                    last_error = "USB/IP or Hybrid listener is not ready".into();
                    for address in std::iter::once(endpoints.usb).chain(endpoints.hybrid) {
                        if !listener_ready(connect_address(address)).await {
                            ready = false;
                            break;
                        }
                    }
                    if ready {
                        // A healthy endpoint alone must not hide an exited child.
                        if let Some(status) = process.status()? {
                            return Err(AuthenticatorServerProcessError::ExitedEarly {
                                status,
                                log_tail: process.log_tail(),
                            });
                        }
                        return Ok(());
                    }
                }
            }
            if Instant::now() >= deadline {
                return Err(AuthenticatorServerProcessError::StartupTimedOut {
                    last_error,
                    log_tail: process.log_tail(),
                });
            }
            sleep(Duration::from_millis(25)).await;
        }
    };
    match timeout(deadline_after, check).await {
        Ok(result) => result,
        Err(_) => Err(AuthenticatorServerProcessError::StartupTimedOut {
            last_error: "readiness check exceeded startup deadline".into(),
            log_tail: process.log_tail(),
        }),
    }
}
async fn listener_ready(address: SocketAddr) -> bool {
    matches!(
        timeout(Duration::from_millis(250), TcpStream::connect(address)).await,
        Ok(Ok(_))
    )
}
