# keyringd-test-usbip

Provides build, start, HTTP control of the virtual AuthenticatorServer, and provides USB/IP HID Backend. Commonly used from FIDO consolidation and UI tests on Linux/macOS/Windows.

## Choose an API

| MODULE | Duty |
| --- | --- |
| `authenticator_server::build::AuthenticatorServerBuild` | Prepare Fat Jar from Gradle |
| `authenticator_server::certificate::HybridTestCertificate` | Generate Test CA and Server Certificates |
| `authenticator_server::manager::AuthenticatorServerManager` | Java Process, Ready Wait, Stop/Collect |
| `authenticator_server::client::AuthenticatorServerClient` | HTTP control |
| `authenticator_server::client::AuthenticatorHandle` | Operate a device and a Consent |
| `usbip_hid_backend::UsbIpHidBackend` | Connect to FIDO HID from DevList/Import |

Build, Certificate, HTTP Client and HID Backend are independent entrances. Manager combines startup settings and health checks, OS Runner owns the entire resource and cleanup.

## Server lifecycle

Use the JDK and Repository Gradle Wrapper. Select from `JAVA_HOME` for Java, and from PATH if not set. No pre-build or JAR specification in environment variables is required.

```rust,no_run
use keyringd_test_usbip::authenticator_server::build::AuthenticatorServerBuild;
use keyringd_test_usbip::authenticator_server::manager::{
    AuthenticatorServerManager, AuthenticatorServerOptions,
};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let jar = AuthenticatorServerBuild::new("/path/to/keyringd").prepare()?;
let options = AuthenticatorServerOptions::new(jar, "target/authenticator.log".into());
let mut server = AuthenticatorServerManager::start(options).await?;
let key = server.client().create_authenticator(Default::default()).await?;
key.delete().await?;
server.stop()?;
# Ok(())
# }
```

`prepare()` is a synchronization API, which is called during environment preparation. The output is `target/authenticator-server/`. Gradle manages incremental builds and Project Cache locks. The JDK can be specified with `with_java_home`.

Server starts by default with a Loopback free port and an empty Device list. Ready waits check HTTP Health, USB/IP, or any Hybrid Listener. Start failures include logs and diagnostics. `stop` and DROP stop and WAIT PROCESS to collect. The HTTP Client to an external server does not have a process outage.

## Device and Consent

`create_authenticator` returns the handle of the key with an independent Credential Store. Since DROP does not remove the device, it is `delete()` with Case Cleanup.

| Operation | Effect |
| --- | --- |
| `attach` / `detach` | Change connection state leaving Credential |
| `configure` | Change the UV/PIN settings and reset the Credential |
| `use_manual_approval` / `use_automatic_approval` | Change the Consent method |
| `pending_requests` / `wait_for_pending_consent` | Observe operation standby |
| `approve` / `reject` | Respond to one Consent |
| `scan_hybrid_qr` | Pass screen QR to virtual phone |
| `delete` | Close connection and discard Device |

The HTTP API and Server implementation is [Authenticator-Server](authenticator-server/README.md). FIDO PIN/UV and responses are handled by WebAuthN4J CTAP, and HTTP approval supplies only CONSENT.

## Userspace USB/IP HID

Wrap `UsbIpHidBackend::new([socket_address, ...])` in `UsbBackend::Hid(Arc::new(backend))` and pass it to `FidoClient::new(handler).with_usb_backend(usb_backend).with_hybrid_connector(hybrid_connector)`. No USB Attach or USB Controller Driver is required to the OS, and Windows can verify common CTAP routes.

Backend enumerates the per-server DevList and sets the connected destination to the BUS ID to identify the Device. Open imports and validates the Descriptor, FIDO Usage, and Interrupt Endpoint. Packet I/O owns the connection and disconnects with DROP.

The transfer is associated with a sequence number. Waiting Future abandon sends Unlink and collects Payload even if the CANCEL and response conflicts. PIN/UV, PRESENCE notifications, and multiple key adoptions are handled by the actual FidoClient.

The target is a single HID Interface and Interrupt In/Out Export of Testbed. The standard for USB/IP Wireless is [Linux Kernel Protocol Documentation](https://docs.kernel.org/usb/usbip_protocol.html). Do not extend to the range of general-purpose USB Driver.

## Hybrid Certificate

`HybridTestCertificate::generate(directory)` creates 7 days valid CA and Server Identity for `cable.ua5v.com` / `127.0.0.1`. Returns Certificate Chain PEM, PKCS#8 Key PEM, CA PEM for OS, and CA DER for Rustls. Do not save CA Private Key. The UNIX Server Private Key is saved at 0600.

Caller manages the save destination and cleanup and passes it to the manager with `HybridTunnelOptions`. Kotlin reads PEM and builds the TLS Keystore in memory. KeyTool/OpenSSL is not required for this certificate generation.

## Verification

```sh
cargo test -p keyringd-test-usbip
```

Explicitly select a real server test using JDK:

```sh
cargo test -p keyringd-test-usbip --test authenticator_server_manager -- --ignored
cargo test -p keyringd-test-usbip --test virtual_authenticators -- --ignored
```

OS USB Attach is each OS Tests, Agent ad delivery is testkit, and the product PasskeyService response injection is owned by Core. Physical Key and Real Phone Warranty check the destination suite readme.
