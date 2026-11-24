use crate::HybridAdvertisement;
use crate::transport::hybrid::tunnel::HybridTunnelEndpoint;

keyringd_testkit::scenario! {
        case "QR Origin Endpoint uses Connect Route" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"

            given "Prepare a Hybrid Tunnel Endpoint with a QR origin Tunnel ID and Domain" {
                let tunnel_id = [0x11; 16];
                let mut plaintext = [0; 16];
                plaintext[11..14].copy_from_slice(&[0xaa, 0xbb, 0xcc]);
        }
        when "Call HybridTunnelEndpoint::from_advertisement" {
            let advertisement = HybridAdvertisement::new(plaintext);
            let endpoint =
                HybridTunnelEndpoint::from_advertisement(&tunnel_id, &advertisement).unwrap();
        }
        then "QR Origin Endpoint uses Connect Route" {
            assert_eq!(endpoint.url(), "wss://cable.ua5v.com/cable/connect/aabbcc/11111111111111111111111111111111");
        }

        }
}
