# Virtual authenticator server

A Kotlin/JVM process exposes a USB/IP listener, an HTTP control API, and an optional Hybrid tunnel. Each virtual authenticator owns its credential store, PIN/UV configuration, connection state, and consent queue. The server starts without devices unless explicitly configured otherwise.

Rust runners build and manage this process through [test-usbip](../README.md). JDK selection uses `JAVA_HOME` or PATH. Dependency revisions are pinned in [gradle/libs.versions.toml](gradle/libs.versions.toml).

## Build and run

From this directory:

```sh
./gradlew --no-daemon --project-cache-dir ../../../target/authenticator-server/gradle-project-cache test buildFatJar
java -jar ../../../target/authenticator-server/libs/keyringd-authenticator-server.jar --usb-port=3240 --control-port=3241
```

The project targets Java 17; CI uses JDK 21. Normal Rust test runs call `AuthenticatorServerBuild::prepare()` instead of requiring a manually built JAR.

Create a device:

```sh
curl -X POST http://127.0.0.1:3241/authenticators \
  -H 'Content-Type: application/json' \
  -d '{"profile":"uv-unconfigured-pin-configured","manualApproval":true}'
```

Profile names combine `uv-configured` / `uv-unconfigured` and `pin-configured` / `pin-unconfigured`. The default is `uv-configured-pin-unconfigured`; the configured test PIN is `1234`.

## Control API

| Endpoint | Behavior |
| --- | --- |
| `GET /authenticators` | List retained devices, including detached devices |
| `POST /authenticators` | Create and connect a device |
| `GET /authenticators/{id}` | Read device state |
| `POST /authenticators/{id}/detach` | Hide the export, close imported connections, cancel pending consent |
| `POST /authenticators/{id}/attach` | Reconnect the same device and retained credentials |
| `DELETE /authenticators/{id}` | Disconnect and discard the device |
| `POST /authenticators/{id}/mode/manual` | Require explicit consent |
| `POST /authenticators/{id}/mode/automatic` | Approve consent automatically |
| `POST /authenticators/{id}/configuration/{uv}/{pin}` | Reset credentials and set verification configuration |
| `GET /authenticators/{id}/requests/pending` | Read pending requests |
| `POST /authenticators/{id}/requests/{requestId}/approve` | Approve one request |
| `POST /authenticators/{id}/requests/{requestId}/reject` | Reject one request |
| `POST /authenticators/{id}/hybrid/scan` | Supply a Hybrid QR URI in the body |

Unknown devices return 404, invalid configurations return 400, and reconfiguration with pending consent returns 409. Attach/detach are idempotent for existing devices. Device IDs and bus IDs are allocated monotonically during the process lifetime.

`--initial-authenticators=N` creates devices at startup. `--manual-approval=true` controls their initial consent mode. Scenarios normally create and delete their own devices through HTTP.

Userspace HID backends observe changes on enumeration. OS USB/IP clients must attach a newly exported or reconnected device explicitly; DEVLIST does not attach devices for them.

## CTAP and consent

WebAuthn4J CTAP owns credential, signature, and PIN/UV processing. This server configures devices and supplies simulated physical consent through its handlers. HTTP approval does not fabricate the CTAP response or bypass PIN verification.

The CTAP `largeBlob` extension is enabled by default. Creation accepts `largeBlobSupport: "largeBlobKey"` for the largeBlobKey extension and authenticatorLargeBlobs command, or `largeBlobSupport: "unsupported"` for scenarios that exercise recovery without large-blob support. Recovery scenarios use PageObjects to respond to two QR prompts when largeBlob is available and three when it is unsupported, then verify successful completion.

The test devices use `alwaysUv=true` with independent PIN and built-in UV readiness. `consentRequests` counts requests reaching a device, including automatic approval, so a case can confirm that the intended virtual key handled an operation.

## Hybrid TLS

Rust's `HybridTestCertificate::generate(directory)` prepares disposable files. Enable the tunnel by supplying both inputs:

```sh
java -jar ../../../target/authenticator-server/libs/keyringd-authenticator-server.jar \
  --hybrid-certificate-chain=/tmp/hybrid-test/hybrid-tunnel-chain.pem \
  --hybrid-private-key=/tmp/hybrid-test/hybrid-tunnel-key.pem
```

The JVM reads a certificate-chain PEM and unencrypted EC PKCS#8 key PEM, then constructs an in-memory TLS KeyStore. The Rust API also returns CA PEM and DER for the caller's trust configuration. It does not save the CA private key.

The caller chooses directories, trust scope, and cleanup. The server models a Phone and test tunnel; it does not establish physical Bluetooth or interoperability with every passkey provider.
