use crate::CtapClient;
use crate::CtapClientError;
use crate::CtapTransportStatus;
use crate::tests::support::transport_usb_ctap_ctap_hid_transport::initialized_channel_with;
use crate::transport::usb::ctap::BROADCAST_CHANNEL;
use crate::transport::usb::ctap::CHANNEL_BUSY_ERROR;
use crate::transport::usb::ctap::CHANNEL_BUSY_RETRY_DELAY;
use crate::transport::usb::ctap::CtapHidTransport;
use crate::transport::usb::ctap::MAX_CHANNEL_BUSY_RETRIES;
use crate::transport::usb::ctap::test_support::InitResponseHid;
use crate::transport::usb::ctap::test_support::InitResponseVariant;
use crate::transport::usb::ctap::test_support::ModelAuthenticatorHid;
use crate::transport::usb::ctap::test_support::ModelHid;
use crate::transport::usb::ctap::test_support::TEST_PACKET_SIZE;
use crate::transport::usb::ctap::test_support::pending_response_hid;
use crate::usb::CtapHidTransportError;
use keyringd_fido::AuthenticatorData;
use keyringd_fido::CtapCommand;
use keyringd_fido::CtapStatusCode;
use keyringd_fido::ctap::AuthenticatorGetAssertionRequest;
use keyringd_fido::ctap::AuthenticatorGetAssertionResponse;
use keyringd_fido::ctap::AuthenticatorMakeCredentialRequest;
use keyringd_fido::ctap::ClientDataHash;
use keyringd_fido::ctap::CtapVersion;
use keyringd_fido::transport::usb::CtapHidCapabilities;
use keyringd_fido::transport::usb::CtapHidCommand;
use keyringd_fido::transport::usb::CtapHidInitResponse;
use keyringd_fido::transport::usb::CtapHidMessage;
use keyringd_fido::transport::usb::CtapHidMessageAssembler;
use keyringd_fido::webauthn::PublicKeyCredentialDescriptor;
use keyringd_fido::webauthn::PublicKeyCredentialParameters;
use keyringd_fido::webauthn::PublicKeyCredentialRpEntity;
use keyringd_fido::webauthn::PublicKeyCredentialType;
use keyringd_fido::webauthn::PublicKeyCredentialUserEntity;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::Ordering;

keyringd_testkit::scenario! {
        case "Fails the complete assertion enumeration when former authenticator state is invalidated" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-arbitration"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "a USB authenticator that loses its pending assertion state between GetAssertion and GetNextAssertion" {
                let channel_id = 0x0102_0304;
                let mut authenticator_data = vec![0x11; 32];
                authenticator_data.extend_from_slice(&[1, 0, 0, 0, 1]);
                let first = AuthenticatorGetAssertionResponse::new(Some(PublicKeyCredentialDescriptor::new(
                        PublicKeyCredentialType::PublicKey,
                        vec![1],
                    )),
                    AuthenticatorData::from_bytes(&authenticator_data).unwrap(),
                    vec![0x30, 1],
                )
                .with_number_of_credentials(2);
                let mut first_payload = vec![CtapStatusCode::Ok.as_byte()];
                first_payload.extend_from_slice(&first.to_cbor_bytes().unwrap());
                let (channel, writes) = initialized_channel_with([
                    CtapHidMessage::new(channel_id, CtapHidCommand::Cbor, first_payload),
                    CtapHidMessage::new(
                        channel_id,
                        CtapHidCommand::Cbor,
                        vec![CtapStatusCode::NotAllowed.as_byte()],
                    ),
                ]);
                let mut client = CtapClient::new(Box::new(channel));
                let request = AuthenticatorGetAssertionRequest::new(
                    "example.com",
                    ClientDataHash::new([7; 32]),
                );
            }
            when "the client requests the complete assertion set" {
                let result = client.get_assertions(&request, CtapVersion::Fido21).await;

            }
            then "the state-loss error fails the complete operation instead of returning the first assertion as a partial success" {
                assert!(matches!(
                    result,
                    Err(CtapClientError::Status(CtapStatusCode::NotAllowed))
                ));
                assert_eq!(writes.load(Ordering::Relaxed), 2);
            }
        }
        case "Completes a credential ceremony through CTAPHID packet framing" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-hid-cbor"
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorMakeCredential"
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetAssertion"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "a CTAP client connected to the stateful model authenticator through a 64-byte CTAPHID packet transport" {
                let io = ModelAuthenticatorHid::new();
                let authenticator = io.authenticator.clone();
                let channel = CtapHidTransport::new(io, None);
                let mut client = CtapClient::new(Box::new(channel));
                let relying_party_id = "example.com";
                let creation_hash = ClientDataHash::new([0x11; 32]);
                let creation_request = AuthenticatorMakeCredentialRequest::new(
                    creation_hash,
                    PublicKeyCredentialRpEntity::new("Example").with_id(relying_party_id),
                    PublicKeyCredentialUserEntity::new("user", vec![1], "User"),
                    vec![PublicKeyCredentialParameters::es256()],
                )
                .with_option("rk", true);
            }
            when "the client obtains authenticator metadata, creates a credential, and requests an assertion for it" {
                let info = client.get_info().await.unwrap();
                let credential = client.make_credential(&creation_request).await.unwrap();
                let credential_id = credential
                    .authenticator_data()
                    .attested_credential_data()
                    .unwrap()
                    .credential_id()
                    .to_vec();
                let assertion_request = AuthenticatorGetAssertionRequest::new(
                    relying_party_id,
                    ClientDataHash::new([0x22; 32]),
                )
                .with_allow_list(vec![PublicKeyCredentialDescriptor::new(
                    PublicKeyCredentialType::PublicKey,
                    credential_id.clone(),
                )]);
                let assertion = client.get_assertion(&assertion_request, CtapVersion::Fido21).await.unwrap();
            }
            then "all three commands cross the CTAP codec and CTAPHID framing boundaries without losing the credential identity" {
                assert_eq!(info.versions(), ["FIDO_2_0"]);
                assert_eq!(assertion.credential().unwrap().id(), credential_id);
                assert!(!assertion.signature().is_empty());
                assert_eq!(
                    authenticator.commands(),
                    [
                        CtapCommand::GetInfo,
                        CtapCommand::MakeCredential,
                        CtapCommand::GetAssertion,
                    ]
                );
            }
        }
        case "Initializes on the broadcast channel and reinitializes after cancellation" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-hid-init"
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-commands"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "an uninitialized CTAPHID channel whose peer verifies that INIT uses the broadcast channel" {
                let io = ModelHid::new();
                let commands = io.commands.clone();
                let statuses = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
                let recorded_statuses = std::sync::Arc::clone(&statuses);
                let mut channel = CtapHidTransport::new(io, Some(std::sync::Arc::new(move |status| {
                    recorded_statuses.lock().unwrap().push(status);
                })));
        }
        when "a CBOR command is sent, the operation is cancelled, and another command is sent" {
            let first_response = channel
                .transmit(&keyringd_fido::CtapRequest::new(
                    CtapCommand::GetInfo,
                    vec![],
                ))
                .await
                .unwrap()
                .to_bytes();
        }
        then "the first CBOR response and its processing notifications are received" {
            assert_eq!(first_response, vec![CtapStatusCode::Ok.as_byte(), 0xa0]);
            assert_eq!(
                *statuses.lock().unwrap(),
                [
                    CtapTransportStatus::Processing,
                    CtapTransportStatus::UserPresenceNeeded
                ]
            );
        }
        when "the channel is cancelled and another CBOR command is sent" {
            channel.cancel().await.unwrap();
            let second_response = channel
                .transmit(&keyringd_fido::CtapRequest::new(
                    CtapCommand::GetInfo,
                    vec![],
                ))
                .await
                .unwrap()
                .to_bytes();
        }
        then "INIT precedes each command, uses the broadcast channel, and KEEPALIVE status remains observable" {
            assert_eq!(second_response, vec![CtapStatusCode::Ok.as_byte(), 0xa0]);
            assert_eq!(
                *commands.lock().unwrap(),
                vec![
                    CtapHidCommand::Init,
                    CtapHidCommand::Cbor,
                    CtapHidCommand::Cancel,
                    CtapHidCommand::Init,
                    CtapHidCommand::Cbor,
                ]
            );
        }

        }
    group "CtapHidTransport::cancel" {
            case "Sends CTAPHID_CANCEL after an in-flight CBOR request is abandoned" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-transaction-abort"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-hid-cancel"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "an initialized HID channel whose authenticator keeps a CBOR request in flight without replying" {
                    let commands = Arc::new(Mutex::new(Vec::new()));
                    let io = pending_response_hid(CtapHidMessageAssembler::new(TEST_PACKET_SIZE).unwrap(), commands.clone());
                    let mut channel = CtapHidTransport::new(io, None);
                    channel.init = Some(
                        CtapHidInitResponse::parse(&[
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            0,
                            1,
                            2,
                            3,
                            4,
                            2,
                            1,
                            0,
                            0,
                            CtapHidCapabilities::CBOR.bits(),
                        ])
                        .unwrap(),
                    );
                }
                when "the pending transmit future is dropped and the channel is cancelled" {
                    let pending_response = {
                        let request = keyringd_fido::CtapRequest::new(CtapCommand::GetInfo, vec![]);
                        let transmit = channel.transmit(&request);
                        tokio::pin!(transmit);
                        tokio::time::timeout(std::time::Duration::from_millis(10), &mut transmit).await
                    };
                }
                then "the CBOR command remains pending until its transmit future is dropped" {
                    assert!(pending_response.is_err());
                }
                when "the channel is cancelled after dropping the transmit future" {
                    channel.cancel().await.unwrap();
                }
                then "CTAPHID_CANCEL follows the abandoned CBOR request on the active channel" {
                    assert_eq!(
                        *commands.lock().unwrap(),
                        vec![CtapHidCommand::Cbor, CtapHidCommand::Cancel]
                    );
                }
            }
    }
    group "CtapHidTransport::initialize" {
            case "Rejects an INIT response whose nonce does not match the request" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-hid-init"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "a HID peer that changes the nonce in its INIT response" {
                    let mut channel = CtapHidTransport::new(InitResponseHid::new(
                        InitResponseVariant::NonceMismatch,
                    ), None);
                }
                when "the CTAPHID channel is initialized" {
                    let result = channel.initialize().await;
                }
                then "initialization fails before the allocated channel can be used" {
                    assert!(matches!(result, Err(CtapHidTransportError::InitNonceMismatch)));
                }
            }
            case "Rejects reserved channel identifiers allocated by INIT" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-hid-init"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "HID peers that allocate the reserved zero and broadcast channel identifiers" {
                    let mut zero_channel = CtapHidTransport::new(InitResponseHid::new(
                        InitResponseVariant::InvalidChannel(0),
                    ), None);
                    let mut broadcast_channel = CtapHidTransport::new(InitResponseHid::new(
                        InitResponseVariant::InvalidChannel(BROADCAST_CHANNEL),
                    ), None);
                }
                when "both CTAPHID channels are initialized" {
                    let zero_result = zero_channel.initialize().await;
                    let broadcast_result = broadcast_channel.initialize().await;
                }
                then "both reserved identifiers are rejected" {
                    assert!(matches!(
                        zero_result,
                        Err(CtapHidTransportError::InvalidAllocatedChannel(0))
                    ));
                    assert!(matches!(
                        broadcast_result,
                        Err(CtapHidTransportError::InvalidAllocatedChannel(BROADCAST_CHANNEL))
                    ));
                }
            }
            case "Rejects an authenticator that does not advertise CTAPHID CBOR" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-hid-init"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "a HID peer whose INIT response omits the CBOR capability" {
                    let mut channel = CtapHidTransport::new(InitResponseHid::new(
                        InitResponseVariant::MissingCborCapability,
                    ), None);
                }
                when "the CTAPHID channel is initialized" {
                    let result = channel.initialize().await;
                }
                then "initialization reports that CTAP2 CBOR is unsupported" {
                    assert!(matches!(result, Err(CtapHidTransportError::CborNotSupported)));
                }
            }
            case "Accepts a future CTAPHID protocol version when the required command is advertised" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-packet-version"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "a HID peer using an unknown future protocol version while advertising CTAPHID CBOR" {
                    let mut channel = CtapHidTransport::new(InitResponseHid::new(
                        InitResponseVariant::FutureProtocolVersion(0xff),
                    ), None);
                }
                when "the CTAPHID channel is initialized" {
                    let response = channel.initialize().await.unwrap();
                }
                then "the host accepts the device and preserves its protocol version" {
                    assert_eq!(response.protocol_version(), 0xff);
                    assert!(response.capabilities().contains(CtapHidCapabilities::CBOR));
                }
            }
    }
    group "CtapHidTransport::transmit" {
            case "Rejects malformed CTAPHID control-response payload lengths" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-hid-error"
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-hid-keepalive"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "initialized HID channels returning empty KEEPALIVE and ERROR payloads" {
                    let channel_id = 0x0102_0304;
                    let mut keepalive_channel = initialized_channel_with([
                        CtapHidMessage::new(channel_id, CtapHidCommand::Keepalive, Vec::new()),
                    ]).0;
                    let mut error_channel = initialized_channel_with([
                        CtapHidMessage::new(channel_id, CtapHidCommand::Error, Vec::new()),
                    ]).0;
                }
                when "a CTAP command receives each malformed control response" {
                    let keepalive_result = keepalive_channel
                        .transmit(&keyringd_fido::CtapRequest::new(CtapCommand::GetInfo, vec![]))
                        .await;
                    let error_result = error_channel.transmit(&keyringd_fido::CtapRequest::new(CtapCommand::GetInfo, vec![])).await;
                }
                then "both responses fail immediately as malformed CTAPHID messages" {
                    assert!(matches!(
                        keepalive_result,
                        Err(CtapHidTransportError::InvalidCommandPayloadLength {
                            command: CtapHidCommand::Keepalive,
                            expected: 1,
                            actual: 0,
                        })
                    ));
                    assert!(matches!(
                        error_result,
                        Err(CtapHidTransportError::InvalidCommandPayloadLength {
                            command: CtapHidCommand::Error,
                            expected: 1,
                            actual: 0,
                        })
                    ));
                }
            }
            case "Distinguishes a cancelled CTAP request from a CTAPHID transport error" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-hid-cancel"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "one HID peer returning CTAP2_ERR_KEEPALIVE_CANCEL in a CBOR response and another returning the same byte in CTAPHID_ERROR" {
                    let channel_id = 0x0102_0304;
                    let mut cbor_client = CtapClient::new(Box::new(initialized_channel_with([
                        CtapHidMessage::new(
                            channel_id,
                            CtapHidCommand::Cbor,
                            vec![CtapStatusCode::KeepaliveCancel.as_byte()],
                        ),
                    ]).0));
                    let mut transport_error_client = CtapClient::new(Box::new(initialized_channel_with([
                        CtapHidMessage::new(
                            channel_id,
                            CtapHidCommand::Error,
                            vec![CtapStatusCode::KeepaliveCancel.as_byte()],
                        ),
                    ]).0));
                }
                when "both clients execute GetInfo" {
                    let cancelled_request = cbor_client.get_info().await;
                    let transport_error = transport_error_client.get_info().await;
                }
                then "only the CBOR response is exposed as the CTAP cancellation status" {
                    assert!(matches!(
                        cancelled_request,
                        Err(CtapClientError::Status(CtapStatusCode::KeepaliveCancel))
                    ));
                    assert!(matches!(
                        transport_error,
                        Err(CtapClientError::Transport(error))
                            if matches!(std::error::Error::source(&error).unwrap()
                                .downcast_ref::<CtapHidTransportError<std::io::Error>>(),
                                Some(CtapHidTransportError::Authenticator(0x2d)))
                    ));
                }
            }
            case "Preserves every assigned CTAPHID error code as a transport error" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-hid-error"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "the assigned non-busy CTAPHID error codes" {
                    let error_codes = [0x01, 0x02, 0x03, 0x04, 0x05, 0x0a, 0x0b, 0x7f];
                    let channel_id = 0x0102_0304;
                }
                when "each code is returned in a one-byte CTAPHID_ERROR response" {
                    let mut results = Vec::new();
                    for error_code in error_codes {
                        let mut channel = initialized_channel_with([
                            CtapHidMessage::new(
                                channel_id,
                                CtapHidCommand::Error,
                                vec![error_code],
                            ),
                        ]).0;
                        results.push((error_code, channel.transmit(&keyringd_fido::CtapRequest::new(CtapCommand::GetInfo, vec![])).await));
                    }
                }
                then "the transport layer preserves each CTAPHID code without interpreting it as a CTAP status" {
                    for (expected, result) in results {
                        assert!(matches!(
                            result,
                            Err(CtapHidTransportError::Authenticator(actual)) if actual == expected
                        ));
                    }
                }
            }
            case "Retries a CTAPHID request when the authenticator reports a busy channel" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-hid-error"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "an initialized HID channel that reports ERR_CHANNEL_BUSY once before returning the CBOR response" {
                    let channel_id = 0x0102_0304;
                    let (mut channel, writes) = initialized_channel_with([
                        CtapHidMessage::new(
                            channel_id,
                            CtapHidCommand::Error,
                            vec![CHANNEL_BUSY_ERROR],
                        ),
                        CtapHidMessage::new(
                            channel_id,
                            CtapHidCommand::Cbor,
                            vec![CtapStatusCode::Ok.as_byte(), 0xa0],
                        ),
                    ]);
                }
                when "the client transmits a GetInfo request" {
                    let started = std::time::Instant::now();
                    let response = channel.transmit(&keyringd_fido::CtapRequest::new(CtapCommand::GetInfo, vec![])).await.unwrap().to_bytes();
                    let elapsed = started.elapsed();

                }
                then "the same request is retransmitted after the busy response and the eventual CBOR response is returned" {
                    assert_eq!(response, vec![CtapStatusCode::Ok.as_byte(), 0xa0]);
                    assert!(elapsed >= CHANNEL_BUSY_RETRY_DELAY);
                    assert_eq!(writes.load(Ordering::Relaxed), 2);
                }
            }
            case "Stops retrying CTAPHID requests after the bounded busy-channel retry budget" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-hid-error"
                async: tokio;
                timeout: std::time::Duration::from_secs(5);

                given "an initialized HID channel that keeps reporting ERR_CHANNEL_BUSY" {
                    let channel_id = 0x0102_0304;
                    let busy_response = || CtapHidMessage::new(
                        channel_id,
                        CtapHidCommand::Error,
                        vec![CHANNEL_BUSY_ERROR],
                    );
                    let (mut channel, writes) = initialized_channel_with(
                        (0..=MAX_CHANNEL_BUSY_RETRIES).map(|_| busy_response()),
                    );
                }
                when "the client exhausts its bounded retry budget" {
                    let result = channel.transmit(&keyringd_fido::CtapRequest::new(CtapCommand::GetInfo, vec![])).await;

                }
                then "the final busy error is surfaced instead of retrying indefinitely" {
                    assert!(matches!(
                        result,
                        Err(CtapHidTransportError::Authenticator(CHANNEL_BUSY_ERROR))
                    ));
                    assert_eq!(writes.load(Ordering::Relaxed), MAX_CHANNEL_BUSY_RETRIES + 1);
                }
            }
    }
}
