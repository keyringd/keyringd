# keyringd-test-authenticator

Test-supported Crate to create CTAP responses in Process in `ModelCtapAuthenticator`. Verify request and state transitions of the FIDO Client using Credential Creation, Signing, PIN/UV and Command Records.

No need for Java, HTTP, USB/IP, or OS HID devices. Communication and OS interoperability are different validation boundaries.

| The boundary you want to verify | Support implementation |
| --- | --- |
| CTAP Command/Response for FIDO Client | The model of this Crate |
| Product Application/Workflow and UI Notifications | Core's TestPasskeyService |
| USB/IP, Hybrid Tunnel, OS UI | [Test-USBIP](../keyringd-test-usbip/README.md) virtual server |

```sh
cargo test -p keyringd-test-authenticator
cargo test -p keyringd-fido-client
```

The client test that uses the model is also on the FIDO-client side. Separately check model self-integrity and real authenticator compatibility.
