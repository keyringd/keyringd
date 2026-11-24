use std::io;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result};

/** The root-owned testbed maps the protocol's implicit port 443 to the phone's unprivileged WSS port. */
pub(super) struct HybridPortForward {
    stop: Arc<AtomicBool>,
    threads: Vec<JoinHandle<()>>,
}

impl HybridPortForward {
    pub(super) fn start() -> Result<Self> {
        let ipv4 = TcpListener::bind("127.0.0.1:443")
            .context("bind IPv4 Hybrid tunnel port 443 on the testbed host")?;
        let ipv6 = TcpListener::bind("[::1]:443")
            .context("bind IPv6 Hybrid tunnel port 443 on the testbed host")?;
        let stop = Arc::new(AtomicBool::new(false));
        let threads = [ipv4, ipv6]
            .into_iter()
            .map(|listener| {
                listener.set_nonblocking(true)?;
                let stopping = Arc::clone(&stop);
                Ok(thread::spawn(move || {
                    while !stopping.load(Ordering::SeqCst) {
                        match listener.accept() {
                            Ok((client, _)) => {
                                thread::spawn(move || {
                                    if let Ok(phone) = TcpStream::connect("127.0.0.1:8443") {
                                        let _ = forward(client, phone);
                                    }
                                });
                            }
                            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                                thread::sleep(Duration::from_millis(25));
                            }
                            Err(error) => {
                                eprintln!("Hybrid tunnel port forward stopped: {error}");
                                break;
                            }
                        }
                    }
                }))
            })
            .collect::<io::Result<Vec<_>>>()?;
        Ok(Self { stop, threads })
    }
}

fn forward(mut client: TcpStream, mut phone: TcpStream) -> io::Result<()> {
    let mut client_to_phone = client.try_clone()?;
    let mut phone_to_client = phone.try_clone()?;
    let sending = thread::spawn(move || {
        let _ = io::copy(&mut client_to_phone, &mut phone);
        let _ = phone.shutdown(Shutdown::Write);
    });
    io::copy(&mut phone_to_client, &mut client)?;
    client.shutdown(Shutdown::Write)?;
    let _ = sending.join();
    Ok(())
}

impl Drop for HybridPortForward {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}
