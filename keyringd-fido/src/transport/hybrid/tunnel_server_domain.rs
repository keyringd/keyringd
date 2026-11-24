use sha2::{Digest, Sha256};

const KNOWN_TUNNEL_DOMAINS: &[&str] = &["cable.ua5v.com", "cable.auth.com"];
const BASE32_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz234567";
const TLDS: &[&str] = &[".com", ".org", ".net", ".info"];

pub(crate) fn decode(encoded: u16) -> Option<String> {
    if encoded < 256 {
        return KNOWN_TUNNEL_DOMAINS
            .get(usize::from(encoded))
            .map(|domain| (*domain).to_owned());
    }
    let mut input = b"caBLEv2 tunnel server domain".to_vec();
    input.extend_from_slice(&encoded.to_le_bytes());
    input.push(0);
    let digest = Sha256::digest(input);
    let mut head = [0u8; 8];
    head.copy_from_slice(&digest[..8]);
    let mut value = u64::from_le_bytes(head);
    let tld = TLDS.get((value & 3) as usize)?;
    value >>= 2;
    let mut domain = String::from("cable.");
    while value != 0 {
        domain.push(*BASE32_CHARS.get((value & 31) as usize)? as char);
        value >>= 5;
    }
    domain.push_str(tld);
    Some(domain)
}
