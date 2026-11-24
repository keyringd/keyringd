# keyringd-fido

Provides specifications and codecs for WebAuthn, CTAP and FIDO Transport. The actual Authenticator operation is [FIDO-Client](../keyringd-fido-client/README.md), and the Passkey is responsible for the use of Passkey.

## Data to be placed

| Module | Content |
| --- | --- |
| `webauthn` | Options, Credential, Client Data, Authenticator Data, Attestation, COSE Public Key |
| `ctap` | GetInfo, MakeCredential, GetAssertion, GetNextAssertion, ClientPin Request/Respond Status, Verified PIN |
| `transport::usb` | Ctaphid Message, Packet Split/Rebuild, Init and Capability |
| `transport::hybrid` | QR Payload, Advertisement, Tunnel Exchange Message |
| `codec` | JSON/CBOR conversion inside the Crate |

Put a meaningless type here as a specification value. OS APIs, Tokio, Hid Handle, Ble Scanner, Noise Session, and UI Interaction are owned by Client or OS Crate.

## Use API

Use each public type Constructor, Getter, `with_*`, `from_*`/`to_*`. Do not expose the codec library value type or internal reader to the caller. The specific Field and Conversion API is [Source](src) positive.

- Use `PublicKeyCredentialCreationOptions`/`PublicKeyCredentialRequestOptions` on the WebAuthn boundary.
- Reuse the same type when CTAP refers to Entity or Descriptor in WebAuthn.
- `AuthenticatorGetAssertionResponse` represents a wire response with omission in CTAP 2.0 and `credential` is `Option`. The Client is responsible for the request's allowList completion and the result type after CREDENTIAL's validation.
- The Credential public key and the PIN/UV Key Agreement keys are represented by type with specific constraints.
- `ctap::Pin` normalizes to Unicode NFC and verifies the number of characters, the number of UTF-8 bytes, and the trailing NUL. You can also specify the minimum number of characters for the authenticator, and the failures are returned in `ctap::PinValidationError`. The row of bytes of PINs to be retained is erased when discarded.
- CTAPHID CODEC handles Packets and does not read/write HID equipment.

## type and error

Keep the distinction between Required, Optional, Default, and Unknowledge. Do not create parallel DTOs that have dropped the corresponding range of Wire Field. Distinguish between any bytes and UTF-8 text. Unknown values that can be retained in the specifications are saved in a reciprocating form.

| Error | Meaning |
| --- | --- |
| `CodecError` | Failed to read/write expressions |
| `ProtocolError` | Required values and combinations do not fit the constraints of the specifications |
| `TransportError` | The structure of ctaphid / Hybrid Message does not hold |

The type is one type per file, and the file name is derived from the type name with SNAKE_CASE.

## Correspondence and verification

The protocol types define which values can be represented. The client defines which operations it supports. When adding a type, test codec round trips, invalid input, and unknown values. Core and the client decide when to generate or use those values. See the [installation guide](https://keyringd.com/installation/) for product requirements.

```sh
cargo test -p keyringd-fido
```

Verification of each specification can be found in [src/tests/spec](src/tests/spec) and [Tests/Spec](tests/spec)). Requirements and case correspondence are managed with [requirements.yaml](../docs/internal/requirements.yaml). Changes to I/O, Timeout, and Cancel are verified on the [FIDO-Client](../keyringd-fido-client/README.md) side.
