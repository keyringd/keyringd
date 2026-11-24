use crate::HybridOperationHint;
use crate::tests::support::transport_hybrid_hybrid_qr_initiated_transaction::digit_decode;
use crate::tests::support::transport_hybrid_hybrid_qr_initiated_transaction::digit_encode;
use crate::tests::support::transport_hybrid_hybrid_qr_initiated_transaction::validate_qr_uri;
use crate::transport::hybrid::HybridQrInitiatedTransaction;
use crate::transport::hybrid::test_support::HybridQrDecodeError;
use keyringd_fido::transport::hybrid::HybridQrPayload;

keyringd_testkit::scenario! {
        case "Numeric Encoding Matches Hybrid Vector" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"

            verify "Numeric Encoding Matches Hybrid Vector" {
                assert_eq!(digit_encode(b"hello world"), "335311851610699281684828783");
                }

        }
        case "Creates a fresh Hybrid QR transaction with the scheme prefix" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"

            given "Generate the first QR Transaction from the same Operation Hint" {
                let first = HybridQrInitiatedTransaction::new(HybridOperationHint::GetAssertion).unwrap();
        }
        when "Generate a second QR Transaction from the same Operation Hint" {
            let second = HybridQrInitiatedTransaction::new(HybridOperationHint::GetAssertion).unwrap();
            let first_payload = HybridQrPayload::from_cbor_bytes(
                &digit_decode(&first.qr_uri()[6..]).unwrap(),
            )
            .unwrap();
        }
        then "Temporary QRs with Scheme and New State" {
            assert!(first.qr_uri().starts_with("FIDO:/"));
            assert!(
                first.qr_uri()[6..]
                    .bytes()
                    .all(|byte| byte.is_ascii_digit())
            );
            assert_ne!(first.qr_uri(), second.qr_uri());
            assert_ne!(first.tunnel_id().unwrap(), second.tunnel_id().unwrap());
            assert_eq!(first.qr_secret.len(), 16);
            assert_ne!(first.qr_secret, second.qr_secret);
            assert_eq!(first_payload.public_key().len(), 33);
            assert!(matches!(first_payload.public_key()[0], 0x02 | 0x03));
            p256::PublicKey::from_sec1_bytes(first_payload.public_key()).unwrap();
            validate_qr_uri(first.qr_uri()).unwrap();
            validate_qr_uri(second.qr_uri()).unwrap();
        }

        }
        case "Rejects noncanonical or invalid numeric payloads" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"

            given "Prepare rogue Scheme, non-canonical number chunk, too short FIDO payload" {
                let wrong_scheme = "https://example.com";
                let noncanonical = "99999999999999999";
                let truncated = "FIDO:/123";
        }
        when "Validate or decode each QR Payload" {
            let scheme_result = validate_qr_uri(wrong_scheme);
            let digit_result = digit_decode(noncanonical);
            let truncated_result = validate_qr_uri(truncated);
        }
        then "Rejected as each formal violation" {
            assert_eq!(
                scheme_result,
                Err(HybridQrDecodeError::InvalidScheme)
            );
            assert_eq!(
                digit_result,
                Err(HybridQrDecodeError::NonCanonicalDigitChunk)
            );
            assert!(truncated_result.is_err());
        }

        }
}
