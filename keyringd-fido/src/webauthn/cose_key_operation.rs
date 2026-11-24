#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CoseKeyOperation(i64);

impl CoseKeyOperation {
    pub const VERIFY: Self = Self(2);

    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> i64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "CoseKeyOperation::value" {
            case "Uses the registered COSE key-operation number for verification" {
                why "This guards the numeric value of a reusable COSE registry primitive from RFC 9052; CTAP 2.2 and WebAuthn Level 3 requirements consume the encoded key but do not independently require keyringd to expose this internal constant."
                spec "https://www.rfc-editor.org/rfc/rfc9052.html#section-7.1"
                verify "the verify key operation is represented by value two" {
                    assert_eq!(CoseKeyOperation::VERIFY.value(), 2);
                }
            }
        }
    }
}
