use crate::{TransportError, transport::hybrid::HybridPlaintextMessage};

keyringd_testkit::scenario! {
        case "You can convert messages back and forth as byte columns" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-hybrid"

            given "Prepare a Hybrid Plaintext Message for Shutdown, CTAP, and Update" {
                let update = vec![0xa0];
                let messages = [
                    HybridPlaintextMessage::Shutdown,
                    HybridPlaintextMessage::Ctap(vec![4, 5]),
                    HybridPlaintextMessage::Update(update),
                ];
            }
            when "Encode each message to the byte column and then decode" {
                let encoded = messages
                    .iter()
                    .map(|message| message.to_bytes().unwrap())
                    .collect::<Vec<_>>();
                let decoded = encoded
                    .iter()
                    .map(|message| HybridPlaintextMessage::from_bytes(message).unwrap())
                    .collect::<Vec<_>>();
            }
            then "Type Byte is 0, 1, 2, all restored to the original message" {
                assert_eq!(encoded, [vec![0], vec![1, 4, 5], vec![2, 0xa0]]);
                assert_eq!(decoded, messages);
                assert_eq!(
                    HybridPlaintextMessage::from_bytes(&[]),
                    Err(TransportError::InvalidHybridFrame)
                );
                assert_eq!(
                    HybridPlaintextMessage::from_bytes(&[1]),
                    Err(TransportError::InvalidHybridFrame)
                );
                assert_eq!(
                    HybridPlaintextMessage::from_bytes(&[3]),
                    Err(TransportError::UnsupportedHybridMessageType(3))
                );
            }

        }
}
