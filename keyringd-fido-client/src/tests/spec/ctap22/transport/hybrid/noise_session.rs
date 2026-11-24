use crate::HybridOperationHint;
use crate::HybridQrInitiatedTransaction;
use crate::transport::hybrid::ctap::CTAP_MESSAGE_TYPE;
use crate::transport::hybrid::ctap::HybridDataChannel;
use crate::transport::hybrid::ctap::HybridNoiseSession;
use crate::transport::hybrid::ctap::PADDING_GRANULARITY;
use crate::transport::hybrid::ctap::pad_plaintext;
use crate::transport::hybrid::ctap::strip_padding;
use crate::transport::hybrid::ctap::test_support::ResponderChannel;

keyringd_testkit::scenario! {
        case "QR Noise Handshake and CTAP Frame interoperable" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "Prepare a Noise Session for Initiator and Authenticator sharing a QR-derived secret" {
                let transaction =
                    HybridQrInitiatedTransaction::new(HybridOperationHint::GetAssertion).unwrap();
                let psk = [0x55; 32];
                let mut channel = ResponderChannel::new(&transaction, &psk);
                let mut handshake = HybridNoiseSession::handshake(&transaction.private_key_bytes(), &psk).unwrap();
                let mut initial = vec![0; 1024];
                let length = handshake.write_message(&[], &mut initial).unwrap();
                initial.truncate(length);
                channel.send(initial).await.unwrap();
                let response = channel.receive().await.unwrap().unwrap();
                let mut initiator = HybridNoiseSession::from_handshake(handshake, &response).unwrap();
                let request = [0x04];
                let encrypted = initiator.encrypt_ctap(&request).unwrap();
                let responder = channel.transport.as_mut().unwrap();
                let mut plaintext = vec![0; encrypted.len()];
        }
        when "Call Responder.Read_Message" {
            let length = responder.read_message(&encrypted, &mut plaintext).unwrap();
            plaintext.truncate(length);
        }
        then "QR Noise Handshake and CTAP Frame interoperable" {
            assert_eq!(plaintext.len() % PADDING_GRANULARITY, 0);
            strip_padding(&mut plaintext).unwrap();
            assert_eq!(plaintext, [CTAP_MESSAGE_TYPE, 0x04]);
            let mut response = vec![CTAP_MESSAGE_TYPE, 0x00, 0xa0];
            response.resize(33, 0);
            response[32] = 29;
            let mut encrypted_response = vec![0; response.len() + 16];
            let length = responder
                .write_message(&response, &mut encrypted_response)
                .unwrap();
            encrypted_response.truncate(length);
            assert_eq!(
                initiator
                    .decrypt_ctap(&encrypted_response)
                    .unwrap(),
                [0x00, 0xa0]
            );
        }

        }

        case "Padding Tunnel Plaintext in 32 bytes" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"

            given "Prepare a 3Byte Tunnel Plaintext" {
                let mut plaintext = vec![1, 4, 5];
            }
            when "Apply Padding" {
                pad_plaintext(&mut plaintext).unwrap();
            }
            then "Becomes 32 bytes and the final byte indicates the leading padding length of 28 bytes." {
                assert_eq!(plaintext.len(), 32);
                assert_eq!(&plaintext[..3], &[1, 4, 5]);
                assert_eq!(&plaintext[3..31], &[0; 28]);
                assert_eq!(plaintext[31], 28);

                plaintext[3..31].fill(0xa5);
                strip_padding(&mut plaintext).unwrap();
                assert_eq!(plaintext, [1, 4, 5]);
            }
        }
}
