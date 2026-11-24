# keyringd-ssh-agent-protocol

A Rust Crate that provides server handling and SSH Wire Value of SSH Agent. The OS provides a connection and the product Handler decides the public identity and signature policy.

```text
OS listener → SshAgent → SshAgentConnection → SshAgentRequestHandler
                         framing / codec      product policy
```

## Built-in boundary

| API | Duty |
| --- | --- |
| `SshAgentListener` | Accept the Stream of `AsyncRead + AsyncWrite` |
| `SshAgent::new(listener, handler)` | Manage Accept Loop and Connect Tasks |
| `SshAgentRequestHandler` | Product boundaries returning a Response from Request |
| `SshAgentConnection` | One Stream Framing, Queue, Handler Call |
| `SshAgentConfig` | Setting the Message, Queue, and Number of Connections |

`start()` runs continuously, and `start_until(shutdown)` runs to Shutdown Future. You can set the upper limit with `with_config` and monitor the number of connections with `subscribe_active_connections`.

Unix Socket, Named Pipe, Socket Activation, and ACLs are owned by each OS Crate. Profile, FIDO, Hardware Key, persistence owned by Core and OS Crate.

## Wire model

Works with 4-Byte Length Prefix and Message Payload, and decodes the Identities, Sign and Extension. Other types are passed to Handler as unknown. Keyringd's Handler sets the unsupported request to FAILURE.

| Model | Content |
| --- | --- |
| `SshPublicKeyBlob` | Agent's Key Blob. A type that holds the original wire-byte sequence of the public key/certificate |
| `SshPublicKey` | Normal public key that has been supported. Keep ECDSA Q, SK-ECDSA Q and Application |
| `SshKeyAlgorithm` | Wire Algorithm identifier for public key and certificate |
| `SshSignature` | ECDSA / SK ECDSA / WebAuthn Enum that holds the signature type of SK ECDSA. Unknown Algorithm keeps its name and data |
| `EcdsaP256SshSignature` / Security-Key Signature type | Structured data such as signature values, flags, counters, and Wire Codec by format |
| `SshCertificate` | Certificate and signature analyzing the key to be certified, public key of CA, validity period, etc. |
| `SshCertificateBody` | ECDSA/SK-ECDSA certificate body enum. Each type holds Application for Nonce, Q, and SK directly |
| `SshCertificateFields` | Common fields such as Serial, validity period, and CA public key that follow the target key |
| `SshCertificateSigningRequest` | Certificates after verification, and the confirmed byte sequence of the signed to be signed |
| `SshSigningPayload` | Structure inspection of SSH Userauth / SSHSIG SIGNED-DATA |

The public key Q holds the original encoding as a validated SEC1 byte sequence, and the SK public key also holds the application. The certificate is represented by the structured `SshCertificate`. Analyze with `from_bytes()` and encode to SSH Wire format with `to_bytes()`.

The `public key blob` of RFC 4252 §7 also includes a certificate. The Agent field name is `key blob` (RFC 9987 §5.5–5.6). The type name `PublicKey` corresponds to the term in this specification.

The unknown Algorithm public key holds the original wire byte sequence in `SshPublicKeyBlob`. Certificate Parsing validates the structure and does not perform any cryptographic verification of the CA-signed.

The Agent Identity and the signature request `key_blob()` returns the `SshPublicKeyBlob` that holds the original wire byte string. When generating a blob, check only the leading algorithm name and do not parse the key-specific fields or certificates. Refer to the original byte sequence in `as_bytes()` and explicitly call `SshPublicKey::from_bytes(blob.as_bytes())` or `SshCertificate::from_bytes(blob.as_bytes())` where needed. The parsed public key `into_key_blob()` and the certificate `to_key_blob()` convert to a blob that holds encoded bytes.

The certificate body stores `EcdsaP256CertificateBody` or `SkEcdsaP256CertificateBody` to the corresponding VARIANT. The target key is expressed as an inline field of the Q/Application, which is determined by the type of certificate, not the public key BLOB. `signature_key` is a `SshPublicKey` representing a supported normal public key and does not store certificates or unsupported formats. The certificate parsing with unsupported CA keys by `SshCertificate` explicitly causes an error, but the certificate can also hold the original byte sequence as `SshPublicKeyBlob`.

When issuing from an existing public key, configure the body with `SshCertificateBody::from_public_key(nonce, key, fields)`, and use `prepare()` to verify and confirm the subject of the signature. Add a signature to `signing_bytes()` of the returned request with `with_signature()`.

WebAuthn is a signature format, not an independent public key or certificate type. Unsupported certificate types also retain the entire agent's blob uninterpreted. The `host_key_blob` of host-bound authentication is also retained in this type and does not make it a condition of the signature request that the CA key of the server certificate can be parsed.

Signature Payload parser validates the format. The product handler determines which Profile Identity should allow which Payload is to be placed on the Core. Use public `from_bytes`/`to_bytes` instead of CODEC internal reader/writer.

## Default limit

| Establishment | Value |
| --- | --- |
| Payload upper limit | 256 KiB |
| Subsequent Requests held during processing | 32 |
| Simultaneous connection | 64 |

Maintain the request and response order of a single connection. The connections are handled by independent tasks.

## Placement and validation

`src/connection/` has Stream processing, `src/protocol/` has a REQUEST/RESPONSE, key, signature, certificate, signature payload and codec. The file name that defines the type should be the type name SNAKE_CASE.

```sh
cargo test -p keyringd-ssh-agent-protocol
```

[Tests/Spec/RFC9987](tests/spec/rfc9987) verifies the specification of Wire, Framing, Order, and Multiple Connections. [Tests/Connection](tests/connection) validates the upper limit and non-compliant requests on the implementation. Fixed Wire Vector is produced with [tests/support/wire.rs](tests/support/wire.rs) without production codec.

Refer to [RFC 9987 correspondence table](rfc9987-conformance-matrix.md) for correspondence status and differences including product and OS.
