# keyringd-fido-client

Asynchronous Clients that perform Authenticator operations from WebAuthn Options. Specification type codec is [FIDO](../keyringd-fido/README.md), and CORE owns the decision of Profile Workflow and SSH usage.

## public entrance

Start operations through [FidoClient](src/client/fido_client.rs).

| API | Contract |
| --- | --- |
| FidoClient::new(handler) | Create a client that is not connected to a route |
| with_usb_backend / with_hybrid_connector / with_internal_authenticator | Connect the route to be used when building |
| make_credential(options, context: MakeCredentialContext) | Returns the registration result and the device ID of the USB HID route |
| get_assertions(options, context: GetAssertionsContext) | Assertion group, client data, return a route |
| cancel(operation_id) | Request Cancel for a specified running operation |

ClientDataProvider borrows for each operation and creates ClientData from Challenge. You can also pass the corresponding Closure. It is generated once when it is ready to be input and is retained by the operational context. Do not reconfigure CTAP requests with UI or Transport.

Returns BUSY during the same Client operation and continues the existing operation. Even after Cancel, the next operation will not be accepted until the cleanup is finished. If you have options.timeout, use that value and use default_timeout (120 seconds) only when not specified. WebAuthn timeout range is not corrected for browsers.

## Connection with Internal Authenticator

If you implement InternalAuthenticator and connect to FidoClient, you can call it from the Client-Device Hint via the Internal route. The boundaries between make_credential and get_assertion are AuthenticatorMakeCredentialRequest/response for FIDO crates, respectively. AuthenticatorGetAssertionRequest/Response. App-specific operational contexts are retained in the Authenticator implementation and not included in the CTAP request.

~~~rust
let options = options.with_hints(vec![PublicKeyCredentialHint::client_device()]);
let assertions = FidoClient::new(())
    .with_internal_authenticator(authenticator)
    .get_assertions(options, GetAssertionsContext::new(&client_data_provider))
    .await?;
~~~

The same FidoClient for USB/HYBRID/Internal manages the life of getting Client Data, preparing CTAP requests, and validating responses. Rp and Credential are specified in OPTIONS, and the signature byte sequences are algorithm-independent. Profile, SSH signature format, OS private key handle is not included in the library's request type.

GetAssertionsResult returns a response set, client data, and a single GetAssertionResult with a SELECT. InternalAuthenticator receives MakeCredential and GetAssertion, and the scope of support is determined by each implementation. DeviceKeyFidoAuthenticator supports GetAssertion and makesCredential returns CTAP InvalidCommand.

Separately from WebAuthn Options, registration takes MakeCredentialContext and authentication receives GetAssertionsContext. Specify the Client Data Provider and any USB device ID for both contexts. Registration is mandatory, and you can specify this UP request only for authentication context. UV requests are specified in WebAuthn Options. The default value for the UP request is TRUE. For non-WebAuthn uses, if the internal Authenticator is not interactive, the GetAssertionsContext::new for each operation Pass (&client_data_provider).with_user_presence(false). Client does not keep UP settings. The Authenticator will receive a cancellation on the passed cancellationToken and return it after the native processing and UI exit. The Client will notify the backend even when it's expired and keep it BUSY until it's finished. Algorithm-specific interpretation and signature validation of the signature is the responsibility of the user with the Credential public key.

## Backend and route

| Backend | Duty |
| --- | --- |
| UsbBackend::Hid / UsbHidBackend | Equipment enumeration, OPEN, PACKET I/O. CTAphiD, PIN/UV, candidate selection is on the Client side |
| UsbBackend::Os / OsUsbBackend | Delegate registration/certification/Cancel to OS API |
| SystemHybridChannelConnector | BLE ad exploration and collation, connection to a Tunnel |

The OS implementation interface is published from [Backend](src/client/backend/mod.rs). The asynchronous API of HID I/O and common Transport uses BoxFuture and keeps the Backend-specific Error as the Common Error Source.

Linux/macOS product USB route is `HidApiBackend`, Windows is `WindowsWebAuthnBackend`. Hybrid uses the common Client on all OSs. Use SystemHybridChannelConnector::default() for normal connection and set an external ad relay with remote_advertisements. The connection destination/CA specification of the test tunnel is published only in Test-Support Feature.

| options.hints | Selection |
| --- | --- |
| Security-Key only | USB |
| Hybrid only | Hybrid |
| Unspecified/unknown HINT only/Multiple support HINT | Request route selection for Handler |
| Only Client-Device | Connected Internal Authenticator (NoAuthenticator if not connected) |

Ignore the unknown HINT and use it if there is a HINT that is supported. Get_Assertions, which clearly states the USB device ID, uses that USB route. Simultaneous designation with client-device is denied as a contradiction. The Client does not remember the choice between operations, and Core Workflow takes over the route/USB device ID from the first success results.

## Interaction and Cancel

When selecting from multiple USB devices by touch, use AuthenticatorSelection after CTAP 2.1 (including 2.1 PRE). In CTAP 2.0, only UP is requested by random RP ID/Credential ID, and only UP is requested, and NO_CREDENTIALS after touch is selected.

The FidoInteractionHandler is responsible for OperationStatus notifications, route selection, and existing PIN/new PIN input. To cancel, use the OperationStarted ID and request the Client from the app. BackendFinished notifies you of the backend processing and the completion of the cleanup. When closing the UI, the machine and termination confirmations are managed by the caller and separated from the Client's completion notice.

The PIN uses FIDO's ctap::Pin to rely on the type to normalize, verify, and erase when discarding. The answer to the inquiry returns the original InteractionID and refuses the old answer. The Client manages PIN matching/settings, number of attempts, and acquisition of Tokens.

QR is a status notification by HybridQRREADY. Windows, wording, and owner are determined by the OS UI. The app's USB button cancels the current HYBRID operation, waits for the cleanup, and then reruns it with USB Hint. Do not search for USB and Hybrid in parallel in the Client.

OperationContext deadlines include interaction and I/O. Candidates use the parent's child, Cancel Token. OS Backend owns a bridge to the OS Cancel API and wait for the blocking process to finish.

## Results and validation boundaries

To verify the GetAssertion response of CTAP, pass the confirmed version from GetInfo on the connection. NumberOFcRedentials with AllowList will only allow 1 of CTAP 2.0 and ask for omissions from 2.1 onwards.

MakeCredentialResult holds WebAuthn registration equivalent information. The CTAP response is converted to a WebAuthn registration response, and the current Backend sets the CrossPlatform Attachment and an empty Client Extension output.

The wire response `AuthenticatorGetAssertionResponse.credential` is kept as an optional `Option`. Client's `AssertionResponse` keeps Credential required. `AssertionResponse::from_ctap` and `GetAssertionsResult::from_ctap` are only complemented by the CREDENTIAL sent in the response, only one case, and returns an error if it cannot be determined. The specified credentials are not overwritten.

GetAssertionsResult holds the AssertionResponse group after the Credential is determined, ClientData, and Transport for common operations. Duplicate empty results and credential IDs are rejected. USB HID's usb_device_id is the device identifier that is connected, and is different from the credential ID. Do not include WebAuthn responses or stored data.

FIDO's type/codec verifies the input specification constraints, and the Client verifies the validity of the corresponding functions, capabilities, and operation results. The client validates the prepared request and the RP ID HASH, the requested UP/UV, the Credential and AllowList correspondence. Signature validation is the responsibility of the user with the public key. The Client is responsible for PIN/UV requests and execution procedures. See [Src](src) for details on getter and public type.

## internal placement

| Portion | what you own |
| --- | --- |
| client/operation | Assembly of ID, Deadline, concurrent execution, route sorting, registration/certification |
| ctap | CTAP Command encoding/transmission/status determination/decoding |
| ctap/pin_uv | PIN/UV Key Exchange, Shared Secret Token, ClientPin Procedure |
| transport/usb | CTaphiD, HID Report analysis, HIDAPI I/O |
| transport/hybrid | QR Transaction, BLE, Cipher, Tunnel, Noise/CTAP Communication |

Common CtapTransport::transmit receives FIDO CTAPREQUEST and returns CTAPRESPONSE. Transport is responsible for FRAMING and CTAP-byte column translation, and CTAPCLIENT is responsible for Status and CBOR. The keepalive notification destination is specified when building the CTAPHID TRANSPORT.

Hybrid Connector is in charge of ad search and tunnel connection, and HybridctapTransport is responsible for Handshake and CTAP communication. Cryptographic status and communication error are separated. BLE Scan's exclusive rights are retained until the end, and external relay and BLE ad collations are grouped together on the BLE side. Retry creates a new QR Transaction.

## Verification during development

```sh
cargo test -p keyringd-fido-client --all-targets --all-features
```

Use [Test-Authenticator](../tests/keyringd-test-authenticator/README.md) for CTAP response in Process, and [Test-USBIP](../tests/keyringd-test-usbip/README.md) for actual Server and USB/IP/HYBRID communication]. Injection of UserSpace HID Backend can be used to verify common CTAP routes on Windows, but the product Windows WebAuthn and physical equipment verification are performed separately.

The Wire Trace policy refers to [Development's Logging](../docs/internal/DEVELOPMENT.md#logging), and the System Test entrance by OS see [Development](../docs/internal/DEVELOPMENT.md)].

Use test_support::MockCtapTransport for CTAP Unit Test and MockHidPacketIO for Packet I/O. Check the expected value of the request/response, the number of cancellations and the order, and transfer from the same mock if you need an authenticator model.
