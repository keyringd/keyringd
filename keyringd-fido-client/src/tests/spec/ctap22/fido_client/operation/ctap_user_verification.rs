use crate::AuthenticatorCapabilityError;
use crate::AuthenticatorRequest;
use crate::AuthenticatorRequirements;
use crate::PinUvAuthProtocol;
use crate::SelectedUserVerification;
use crate::tests::support::client_operation_ctap_ctap_user_verification::check_candidate;
use crate::tests::support::client_operation_ctap_ctap_user_verification::info;
use crate::tests::support::client_operation_ctap_ctap_user_verification::request;
use keyringd_fido::UserVerificationRequirement;
use keyringd_fido::ctap::AuthenticatorGetInfoResponse;
use keyringd_fido::webauthn::PublicKeyCredentialParameters;
use keyringd_fido::webauthn::PublicKeyCredentialType;
use std::collections::BTreeMap;

keyringd_testkit::scenario! {
    group "Authenticator capability checks and PIN/UV selection" {
        case "Selects PIN/UV Auth Protocol 2 when both supported protocols are available" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto2"

            given "an authenticator that advertises PIN/UV Auth Protocol 1 before Protocol 2" {
                let requirements = AuthenticatorRequirements {
                    required_extensions: vec!["credProtect".into()],
                    ..AuthenticatorRequirements::keyringd()
                };
        }
        when "the authenticator capabilities are evaluated using the keyringd platform policy" {
            let result = check_candidate(&info(), &requirements, AuthenticatorRequest::GetAssertion(&request())).unwrap();
        }
        then "Protocol 2 is selected despite being listed after Protocol 1" {
            assert_eq!(
                result.1,
                SelectedUserVerification::ClientPin(PinUvAuthProtocol::Two)
            );
            assert!(result.0.supports_permission_tokens());
        }

        }
        case "Falls back to PIN/UV Auth Protocol 1 when Protocol 2 is unavailable" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#pinProto1"
            given "an authenticator that supports client PIN with only PIN/UV Auth Protocol 1" {
                let candidate = info().with_pin_uv_auth_protocols(vec![1]);
            }
            when "the authenticator capabilities are evaluated" {
                let result = check_candidate(
                    &candidate,
                    &AuthenticatorRequirements::keyringd(), AuthenticatorRequest::GetAssertion(&request()))
                .unwrap();
            }
            then "Protocol 1 is selected as the compatible fallback" {
                assert_eq!(
                    result.1,
                    SelectedUserVerification::ClientPin(PinUvAuthProtocol::One)
                );
            }
        }
        case "Prefers a permission-scoped built-in-UV token over the legacy uv option" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#gettingPinUvAuthToken"
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#makecred-uv"
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getassert-uv"

            given "an authenticator that supports built-in UV, permission-scoped tokens, and PIN/UV Auth Protocol 2" {
                let mut options = info().options().unwrap().clone();
                options.insert("uv".to_owned(), true);
                let candidate = info().with_options(options);
            }
            when "capabilities are evaluated for required user verification" {
                let result = check_candidate(
                    &candidate,
                    &AuthenticatorRequirements {
                        user_verification: UserVerificationRequirement::Required,
                        ..AuthenticatorRequirements::keyringd()
                    }, AuthenticatorRequest::GetAssertion(&request()))
                .unwrap();
            }
            then "the client selects getPinUvAuthTokenUsingUvWithPermissions with Protocol 2" {
                assert_eq!(
                    result.1,
                    SelectedUserVerification::BuiltInUvToken(PinUvAuthProtocol::Two)
                );
                assert_eq!(result.0.preferred_platform_uv_attempts(), 1);
            }
        }
        case "Retains the authenticator's preferred number of platform UV attempts" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-preferredplatformuvattempts"

            given "an authenticator that asks the platform to try built-in UV three times" {
                let mut options = info().options().unwrap().clone();
                options.insert("uv".to_owned(), true);
                let candidate = info()
                    .with_options(options)
                    .with_preferred_platform_uv_attempts(3);
            }
            when "capabilities are evaluated for required user verification" {
                let result = check_candidate(
                    &candidate,
                    &AuthenticatorRequirements {
                        user_verification: UserVerificationRequirement::Required,
                        ..AuthenticatorRequirements::keyringd()
                    }, AuthenticatorRequest::GetAssertion(&request()))
                .unwrap();
            }
            then "the operation coordinator receives the requested three-attempt budget" {
                assert_eq!(result.0.preferred_platform_uv_attempts(), 3);
            }
        }
        case "Uses the legacy uv option when permission-scoped built-in-UV tokens are unavailable" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#makecred-uv"
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getassert-uv"

            given "an authenticator that supports built-in UV but does not advertise pinUvAuthToken" {
                let candidate = AuthenticatorGetInfoResponse::new(
                    vec!["FIDO_2_1".to_owned()],
                    [0; 16],
                )
                .with_options(BTreeMap::from([
                    ("rk".to_owned(), true),
                    ("uv".to_owned(), true),
                ]))
                .with_algorithms(vec![PublicKeyCredentialParameters::es256()]);
            }
            when "capabilities are evaluated for required user verification" {
                let result = check_candidate(
                    &candidate,
                    &AuthenticatorRequirements {
                        user_verification: UserVerificationRequirement::Required,
                        ..AuthenticatorRequirements::keyringd()
                    }, AuthenticatorRequest::GetAssertion(&request()))
                .unwrap();
            }
            then "the client selects the uv option without inventing pinUvAuthParam" {
                assert_eq!(
                    result.1,
                    SelectedUserVerification::BuiltInUvOption
                );
            }
        }
        case "Rejects client PIN when the authenticator advertises no supported PIN/UV Auth Protocol" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorClientPIN"
            given "an authenticator that supports client PIN but advertises only unknown PIN/UV Auth Protocol 3" {
                let candidate = info().with_pin_uv_auth_protocols(vec![3]);
            }
            when "the authenticator capabilities are evaluated" {
                let result = check_candidate(
                    &candidate,
                    &AuthenticatorRequirements::keyringd(), AuthenticatorRequest::GetAssertion(&request()));
            }
            then "capability negotiation reports that the PIN/UV Auth Protocol is unsupported" {
                assert!(matches!(result,
                    Err(AuthenticatorCapabilityError::PinUvAuthProtocolUnsupported)));
            }
        }
        case "Does not use ClientPIN for MakeCredential or GetAssertion when those permissions are unavailable" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#getinfo-nomcgapermissionswithclientpin"
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#gettingPinUvAuthToken"

            given "an authenticator whose ClientPIN tokens cannot receive MakeCredential or GetAssertion permissions" {
                let mut options = info().options().unwrap().clone();
                options.insert("noMcGaPermissionsWithClientPin".to_owned(), true);
                let candidate = info().with_options(options);
                let required = AuthenticatorRequirements {
                    user_verification: UserVerificationRequirement::Required,
                    ..AuthenticatorRequirements::keyringd()
                };
                let discouraged = AuthenticatorRequirements {
                    user_verification: UserVerificationRequirement::Discouraged,
                    ..AuthenticatorRequirements::keyringd()
                };
            }
            when "capabilities are evaluated for required and discouraged user verification" {
                let required_result = check_candidate(&candidate, &required, AuthenticatorRequest::GetAssertion(&request()));
                let discouraged_result =
                    check_candidate(&candidate, &discouraged, AuthenticatorRequest::GetAssertion(&request())).unwrap();
            }
            then "required verification is unavailable and a discouraged request selects no verification" {
                assert!(matches!(required_result,
                    Err(AuthenticatorCapabilityError::UserVerificationUnavailable)));
                assert_eq!(
                    discouraged_result.1,
                    SelectedUserVerification::None
                );
            }
        }
        case "Treats a discouraged RP request as requiring verification when alwaysUv is enabled" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-feature-descriptions-alwaysUv-platf-actions"

            given "an authenticator with alwaysUv, built-in UV, and permission-scoped tokens enabled" {
                let mut options = info().options().unwrap().clone();
                options.insert("alwaysUv".to_owned(), true);
                options.insert("makeCredUvNotRqd".to_owned(), false);
                options.insert("uv".to_owned(), true);
                let candidate = info().with_options(options);
                let requirements = AuthenticatorRequirements {
                    user_verification: UserVerificationRequirement::Discouraged,
                    ..AuthenticatorRequirements::keyringd()
                };
            }
            when "the platform evaluates the RP request against authenticator capabilities" {
                let result = check_candidate(&candidate, &requirements, AuthenticatorRequest::GetAssertion(&request()))
                    .unwrap();
            }
            then "alwaysUv overrides discouraged and selects permission-scoped built-in verification" {
                assert_eq!(
                    result.1,
                    SelectedUserVerification::BuiltInUvToken(PinUvAuthProtocol::Two)
                );
            }
        }
        case "Set the maximum message size to 1024" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"

            given "Prepare the Authenticator ability that does not notify maximum message size" {
                let candidate = AuthenticatorGetInfoResponse::new(vec!["FIDO_2_1".into()], [0; 16])
                    .with_options(BTreeMap::from([("rk".into(), true)]))
                    .with_algorithms(vec![PublicKeyCredentialParameters::es256()]);
        }
        when "Call check_candidate" {
            let result = check_candidate(
                &candidate,
                &AuthenticatorRequirements::keyringd(), AuthenticatorRequest::GetAssertion(&request()))
            .unwrap();
        }
        then "Set the maximum message size to 1024" {
            assert_eq!(result.0.maximum_message_size(), 1024);
        }

        }
        case "Exclude candidates that cannot accept post filter requests" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"

            given "Prepare a missing authenticator and a Credential ID that is missing message size" {
                let small_message_authenticator = info().with_max_message_size(10);
                let descriptor = keyringd_fido::webauthn::PublicKeyCredentialDescriptor::new(
                    PublicKeyCredentialType::PublicKey,
                    vec![1, 2, 3],
                );
                let credential_request = keyringd_fido::ctap::AuthenticatorGetAssertionRequest::new(
                    "example.com", keyringd_fido::ctap::ClientDataHash::new([0; 32]),
                ).with_allow_list(vec![descriptor]);
                let short_credential_authenticator = info().with_max_credential_id_length(2);
        }
        when "Evaluate the capacity of each Authenticator against a request" {
            let message_size_result = check_candidate(
                &small_message_authenticator,
                &AuthenticatorRequirements::keyringd(), AuthenticatorRequest::GetAssertion(&request()));
            let credential_result = check_candidate(
                &short_credential_authenticator,
                &AuthenticatorRequirements::keyringd(), AuthenticatorRequest::GetAssertion(&credential_request));
        }
        then "Message Size Missing Match Credential Excluded from Candidates" {
            assert!(matches!(
                message_size_result,
                Err(AuthenticatorCapabilityError::MessageTooLarge { maximum: 10, .. })
            ));
            assert!(matches!(credential_result,
                Err(AuthenticatorCapabilityError::NoMatchingCredential)));
        }

        }
        case "Reject each unmet mandatory requirement" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"

            given "Prepare a request to evaluate each required requirement for Resident Key, UV, Algorithm, and Extension" {
                let requirements = AuthenticatorRequirements {
                    user_verification: UserVerificationRequirement::Required,
                    required_extensions: vec!["credProtect".into()],
                    ..AuthenticatorRequirements::keyringd()
                };
                let mut options = info().options().unwrap().clone();
                options.insert("rk".into(), false);
        }
        when "Prepare a non-responsible candidate for Resident Key" {
            let candidate = info().with_options(options);
        }
        then "Rejects unsatisfactory requirements and keep the unset Client PIN as a configurable UV method" {
            assert!(matches!(check_candidate(&candidate, &requirements, AuthenticatorRequest::GetAssertion(&request())),
                Err(AuthenticatorCapabilityError::DiscoverableCredentialsUnsupported)));
            let candidate = info().with_extensions(vec![]);
            assert!(matches!(check_candidate(&candidate, &requirements, AuthenticatorRequest::GetAssertion(&request())),
                Err(AuthenticatorCapabilityError::ExtensionUnsupported(extension)) if extension == "credProtect"));
            let candidate = info().with_algorithms(vec![PublicKeyCredentialParameters::new(
                PublicKeyCredentialType::PublicKey,
                keyringd_fido::webauthn::CoseAlgorithmIdentifier::new(-8),
            )]);
            assert!(matches!(check_candidate(&candidate, &requirements, AuthenticatorRequest::GetAssertion(&request())),
                Err(AuthenticatorCapabilityError::AlgorithmUnsupported)));
            let candidate = info().with_options(BTreeMap::from([("rk".into(), true)]));
            assert!(matches!(check_candidate(&candidate, &requirements, AuthenticatorRequest::GetAssertion(&request())),
                Err(AuthenticatorCapabilityError::UserVerificationUnavailable)));
            let mut options = info().options().unwrap().clone();
            options.insert("clientPin".into(), false);
            let candidate = info().with_options(options);
            let capabilities = check_candidate(&candidate, &requirements, AuthenticatorRequest::GetAssertion(&request())).unwrap();
            assert_eq!(
                capabilities.1,
                SelectedUserVerification::ClientPinSetup {
                    protocol: PinUvAuthProtocol::Two,
                    minimum_length: 4,
                }
            );
        }

        }
    }
}
