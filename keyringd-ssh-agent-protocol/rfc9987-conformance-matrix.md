# RFC 9987 Correspondence Table

Organize the scope of the current Agent implementation, the corresponding automatic testing, and known differences. The original specification is [RFC 9987](https://www.rfc-editor.org/rfc/rfc9987.html), and case and requirement ID correspondence is [requirements.yaml](../docs/internal/requirements.yaml). "Test with" in the table indicates the existence of the test, and the pass/fail of the target execution is checked in the report.

## Implementation boundary

Protocol Crate deals with Frame, Request/Response, and connections. Core determines the Identity enumeration and signature permissions from the Profile. Endpoint generation and access rights are handled by the external SSH Client/Server for the OS Crate, and the Transport for SSH Forwarding.

KeyRingd provides the Identity enumeration and signature, key addition/removal and locks are not accepted via the Agent API. These requests are sent to Failure. Profile management is handled by the product UI.

## Connect with Wire

| Target | Implementation/Confirmation Contents | Circumstances | Evidence |
| --- | --- | --- | --- |
| §3 / §5 framing | Length Prefix, Type, Payload, Split/Continuous Frame | There is a test | [connection_framing.rs](tests/spec/rfc9987/connection_framing.rs) |
| §3 response order | Response order per Request, do not send without request | There is a test | [response_ordering.rs](tests/spec/rfc9987/response_ordering.rs) |
| §5.1 generic response | Success/Failure and request-specific Response Wire | There is a test | [wire_messages.rs](tests/spec/rfc9987/wire_messages.rs) |
| §5.1 unsupported type | Pass the unsupported type to the handler and convert the FAILURE to FRAME | There is a test | [unsupported_requests.rs](tests/connection/unsupported_requests.rs) |
| §6 multiple clients | Multiple connections to one agent | There is a test | [multiple_connections.rs](tests/spec/rfc9987/multiple_connections.rs) |
| Implementation upper limit | Payload, Pending Queue, Bad Frame | There is a test | [framing_limits.rs](tests/connection/framing_limits.rs) |

A fixed vector is made independently of Production Codec with [Tests/Support/Wire.rs](tests/support/wire.rs).

## Product Handler

| Target | Product behavior | Circumstances | Evidence |
| --- | --- | --- | --- |
| §5.5 identities | Returns the currently available public identity of the valid profile | There is a test | [identity_listing.rs](../keyringd-core/tests/spec/rfc9987/identity_listing.rs) |
| §5.6 signing | Returns matching Identity signature success and rejects unknown keys and non-Zero Flags | There is a test | [request_handler.rs](../keyringd-core/src/tests/spec/rfc9987/request_handler.rs) |
| §5.2 add / constrained add / token load | Set the non-supported Request to FAILURE | There is a rejection test | Same Handler and [Unsupported_Requests.rs](tests/connection/unsupported_requests.rs) |
| §5.4 remove / remove all / token remove | Set the non-supported Request to FAILURE | There is a rejection test | Same as above |
| §5.7 lock / unlock | Set the non-supported Request to FAILURE | There is a rejection test | Same as above |
| §5.8 unknown extension / query | generic Failure | There is a rejection test | Same Handler and Connection Test |
| PXP advertisement relay | Correct Payload when OPT-IN is enabled is SUCCESS, Incorrect Payload is ExtensionFailure, Disabled is FAILURE | There is a test | Same Handler and Response Wire Test |
| OpenSSH session-bind | Failure without claiming response | There is a rejection test | Same handler |

It distinguishes that the Request Parser can keep unknown messages and the product handler accepts functionality. Since the execution functions of Key Constraints, Token Library Load, and Agent Lock are not implemented, state transitions are not guaranteed when accepted.

## Endpoint

| OS | Evidence | What to check |
| --- | --- | --- |
| Linux | [endpoint.rs](../keyringd-linux/src/tests/spec/rfc9987/endpoint.rs) | UNIX Socket and Placement/Permission |
| macOS | [endpoint.rs](../keyringd-macos/src/tests/spec/rfc9987/endpoint.rs) | UNIX Socket and Placement/Permission |
| Windows | [windows_endpoint_access.rs](../keyringd-windows/src/tests/spec/rfc9987/windows_endpoint_access.rs) | Real Named Pipe User-Only DACL and Subsequent Instance |
| WSL | [endpoint.rs](../keyringd-wsl/src/tests/spec/rfc9987/endpoint.rs) | User Socket and Package Unit |

The OS-specific test runs on that OS. The Duplex Stream Test of Protocol Crate does not guarantee Endpoint ACLs.

## Differences and unverified ranges

PXP Extension follows [RFC §5.8](https://www.rfc-editor.org/rfc/rfc9987.html#section-5.8), and returns extension_failure (28) for invalid payload when valid. Returns Generic Failure (5) for OPT-IN disabled and unknown extensions.

The Unknown Type test verifies the representative values including unregistered TYPEs and 0, 24, 30 and 240. Do not claim that all Reserved/Private-Use Bytes are individually covered.

Forwarding no additional control of Provenance, Destination Constraints, and Visibility/Use keys not implemented. This suite does not include the actual verification of the Side-Channel characteristics. Forwarding permissions and channel processing on the SSH Client/Server side are out of range for Agent Responder.

When changing, add a Test to the owner of the Parser, Handler, and OS Endpoint, and update the table and requirements.
