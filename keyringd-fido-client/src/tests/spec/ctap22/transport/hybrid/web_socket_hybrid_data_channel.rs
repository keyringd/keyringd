use crate::HybridWebSocketError;
use crate::tests::support::transport_hybrid_tunnel_web_socket_hybrid_data_channel::connected_pair;
use crate::transport::hybrid::ctap::HybridDataChannel;
use crate::transport::hybrid::tunnel::resolve_redirect;
use crate::transport::hybrid::tunnel::web_socket_request;
use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::http::HeaderValue;
use tokio_tungstenite::tungstenite::http::header::SEC_WEBSOCKET_PROTOCOL;

keyringd_testkit::scenario! {
        case "Specify FIDO.Cable subprotocol on Hybrid WebSocket connection" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"

            given "Prepare a WebSocket URL for Hybrid Tunnel" {
                let url = "wss://cable.example/cable/connect/010203/040506";
            }
            when "Generate a WebSocket Request with the same function as the production connection process" {
                let request = web_socket_request(url).unwrap();
            }
            then "sec-websocket-protocol matches fido.cable" {
                assert_eq!(
                    request.headers().get(SEC_WEBSOCKET_PROTOCOL),
                    Some(&HeaderValue::from_static("fido.cable"))
                );
            }
        }
        case "Accepts binary Hybrid tunnel messages and rejects text frames" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#hybrid-qr-initiated"
            async: tokio;
            timeout: std::time::Duration::from_secs(5);

            given "an in-process WebSocket tunnel connected to the Hybrid data channel" {
                let (mut channel, mut server) = connected_pair().await;
            }
            when "the tunnel sends one binary message followed by one text message" {
                server.send(Message::Binary(vec![1, 2, 3].into())).await.unwrap();
                let binary = channel.receive().await.unwrap();
                server.send(Message::Text("not binary".into())).await.unwrap();
                let text = channel.receive().await;
            }
            then "the binary message is delivered and the text frame is rejected" {
                assert_eq!(binary, Some(vec![1, 2, 3]));
                assert!(matches!(text, Err(HybridWebSocketError::UnexpectedMessage)));
            }
        }
    group "resolve_redirect" {
            case "Resolve WebSocket Redirect for Absolute URLs and Relative URLs" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-hybrid"

                given "Prepare a relative and absolute redirect for a connected URL" {
                    let base = "wss://cable.example/cable/connect/01/02";
                    let relative = "/replacement/03";
                    let absolute = "wss://other.example/cable/connect/03/04";
            }
            when "Resolve both Redirects" {
                let relative_result = resolve_redirect(base, relative).unwrap();
                let absolute_result = resolve_redirect(base, absolute).unwrap();
            }
            then "Relative URLs are merged to Origin and absolute URLs are preserved as-is" {
                assert_eq!(
                    relative_result,
                    "wss://cable.example/replacement/03"
                );
                assert_eq!(absolute_result, absolute);
            }

            }
            case "Rejects invalid and non-WebSocket redirects" {
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#sctn-hybrid"

                given "Prepare HTTPS Redirect and an Incorrect Source URL" {
                    let https_redirect = "https://other.example/path";
                    let invalid_base = "not a URL";
            }
            when "Resolved as WebSocket Redirect" {
                let scheme_result = resolve_redirect("wss://cable.example/path", https_redirect);
                let base_result = resolve_redirect(invalid_base, "/path");
            }
            then "Both are rejected as illegal redirects" {
                assert!(matches!(
                    scheme_result,
                    Err(HybridWebSocketError::InvalidRedirect(None))
                ));
                assert!(matches!(
                    base_result,
                    Err(HybridWebSocketError::InvalidRedirect(Some(_)))
                ));
            }

            }
    }
}
