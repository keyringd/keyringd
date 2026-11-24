use crate::codec::CborItem as Value;

/// A validated view of a CTAP large-blob map's required fields.
/// The enclosing array retains unknown fields for other clients.
pub struct LargeBlobMap<'a> {
    pub ciphertext: &'a [u8],
    pub nonce: &'a [u8; 12],
    pub orig_size: u64,
}
impl<'a> LargeBlobMap<'a> {
    pub(crate) fn from_value(value: &'a Value) -> Option<Self> {
        let Value::Map(fields) = value else {
            return None;
        };
        let (mut ciphertext, mut nonce, mut orig_size) = (None, None, None);
        for (key, value) in fields {
            match (key, value) {
                (Value::Unsigned(1), Value::ByteString(v)) if ciphertext.is_none() => {
                    ciphertext = Some(v.as_slice())
                }
                (Value::Unsigned(2), Value::ByteString(v)) if nonce.is_none() && v.len() == 12 => {
                    nonce = Some(v.as_slice().try_into().ok()?)
                }
                (Value::Unsigned(3), Value::Unsigned(v)) if orig_size.is_none() => {
                    orig_size = Some(*v)
                }
                (Value::Unsigned(1..=3), _) => return None,
                _ => {}
            }
        }
        Some(Self {
            ciphertext: ciphertext?,
            nonce: nonce?,
            orig_size: orig_size?,
        })
    }
}
