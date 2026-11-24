use crate::HybridAdvertisement;
use crate::HybridCtapTransportError;
use crate::HybridOperationHint;
use crate::HybridQrInitiatedTransaction;
use crate::tests::support::transport_hybrid_ctap_hybrid_ctap_transport::get_info_bytes;
use crate::transport::hybrid::ctap::HybridCtapTransport;
use crate::transport::hybrid::ctap::test_support::ModelHybridChannel;
use keyringd_fido::CtapCommand;
use keyringd_fido::CtapStatusCode;

keyringd_testkit::scenario! {
        case "Cache first get info to transfer CTAP Message" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-hybrid"
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authenticatorGetInfo"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "Prepare a Hybrid Peer that returns Get Info and CTAP responses" {
                let transaction =
                    HybridQrInitiatedTransaction::new(HybridOperationHint::GetAssertion).unwrap();
                let advertisement = HybridAdvertisement::new([0; 16]);
                let get_info = get_info_bytes();
                let model = ModelHybridChannel::new(&transaction, &advertisement, get_info.clone());
                let closed = model.closed.clone();
                let shutdown_frames = model.shutdown_frames.clone();
                let mut channel = HybridCtapTransport::connect(model, &transaction.private_key_bytes(), &transaction.psk(advertisement.plaintext()).unwrap())
                    .await
                    .unwrap();
                let mut expected_info = vec![CtapStatusCode::Ok.as_byte()];
        }
        when "Assemble expected responses from Status Byte and Get Info CBOR" {
            expected_info.extend_from_slice(&get_info);
        }
        then "Verify that the results match CtapStatusCode::Ok" {
            assert_eq!(
                channel.transmit(&keyringd_fido::CtapRequest::new(CtapCommand::GetInfo, vec![])).await.unwrap().to_bytes(),
                expected_info
            );
            assert_eq!(
                channel
                    .transmit(&keyringd_fido::CtapRequest::new(CtapCommand::GetAssertion, vec![0xa0]))
                    .await
                    .unwrap().to_bytes(),
                [CtapStatusCode::Ok.as_byte(), 0xa0]
            );
            channel.cancel().await.unwrap();
            assert!(closed.load(std::sync::atomic::Ordering::SeqCst));
            assert_eq!(shutdown_frames.load(std::sync::atomic::Ordering::SeqCst), 1);
        }

        }
        case "Rejects PEERs that are not CTAP-enabled" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-hybrid"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "Prepare a Hybrid Peer that doesn't show CTAP Feature" {
                let transaction =
                    HybridQrInitiatedTransaction::new(HybridOperationHint::GetAssertion).unwrap();
        }
        when "Call get_info_bytes" {
            let advertisement = HybridAdvertisement::new([0; 16]);
            let model = ModelHybridChannel::new(&transaction, &advertisement, get_info_bytes())
                .with_supported_features(vec!["dc".to_owned()]);
        }
        then "Verify that the results match HybridCtapChannelError::CtapUnsupported" {
            assert!(matches!(
                HybridCtapTransport::connect(model, &transaction.private_key_bytes(), &transaction.psk(advertisement.plaintext()).unwrap()).await,
                Err(HybridCtapTransportError::CtapUnsupported)
            ));
        }

        }
        case "Close Transport with Normal Close" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-hybrid"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "Prepare a Hybrid Transport and a Running Channel that returns a normal response" {
                let transaction =
                    HybridQrInitiatedTransaction::new(HybridOperationHint::GetAssertion).unwrap();
                let advertisement = HybridAdvertisement::new([0; 16]);
                let get_info = get_info_bytes();
                let model = ModelHybridChannel::new(&transaction, &advertisement, get_info);
                let closed = model.closed.clone();
                let shutdown_frames = model.shutdown_frames.clone();
                let mut channel = HybridCtapTransport::connect(model, &transaction.private_key_bytes(), &transaction.psk(advertisement.plaintext()).unwrap())
                    .await
                    .unwrap();
        }
        when "Call channel.close" {
            channel.close().await.unwrap();
        }
        then "Close Transport with Normal Close" {
            assert!(closed.load(std::sync::atomic::Ordering::SeqCst));
            assert_eq!(shutdown_frames.load(std::sync::atomic::Ordering::SeqCst), 1);
        }

        }
        case "Shutdown Closes Transport even when sending fails" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-hybrid"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "Have a Hybrid Transport that fails when you send shutdown" {
                let transaction = HybridQrInitiatedTransaction::new(HybridOperationHint::GetAssertion).unwrap();
                let advertisement = HybridAdvertisement::new([0; 16]);
                let mut model = ModelHybridChannel::new(&transaction, &advertisement, get_info_bytes());
                model.fail_shutdown_send = true;
                let closed = model.closed.clone();
                let shutdown_frames = model.shutdown_frames.clone();
        }
        when "Connect with a Failed Transport" {
            let mut channel = HybridCtapTransport::connect(model, &transaction.private_key_bytes(), &transaction.psk(advertisement.plaintext()).unwrap()).await.unwrap();
        }
        then "Shutdown Closes Transport even when sending fails" {
            assert!(channel.close().await.is_err());
            assert!(closed.load(std::sync::atomic::Ordering::SeqCst));
            assert_eq!(shutdown_frames.load(std::sync::atomic::Ordering::SeqCst), 1);
        }

        }
}
