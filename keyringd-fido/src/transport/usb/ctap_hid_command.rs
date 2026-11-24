#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CtapHidCommand {
    Ping,
    Msg,
    Lock,
    Init,
    Wink,
    Cbor,
    Cancel,
    Keepalive,
    Error,
    Unknown(u8),
}

impl CtapHidCommand {
    pub const fn from_wire(value: u8) -> Self {
        match value & 0x7f {
            1 => Self::Ping,
            3 => Self::Msg,
            4 => Self::Lock,
            6 => Self::Init,
            8 => Self::Wink,
            0x10 => Self::Cbor,
            0x11 => Self::Cancel,
            0x3b => Self::Keepalive,
            0x3f => Self::Error,
            value => Self::Unknown(value),
        }
    }
    pub const fn wire_value(self) -> u8 {
        (match self {
            Self::Ping => 1,
            Self::Msg => 3,
            Self::Lock => 4,
            Self::Init => 6,
            Self::Wink => 8,
            Self::Cbor => 0x10,
            Self::Cancel => 0x11,
            Self::Keepalive => 0x3b,
            Self::Error => 0x3f,
            Self::Unknown(value) => value & 0x7f,
        }) | 0x80
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "CtapHidCommand::from_wire" {
            case "Preserves unassigned CTAPHID command identifiers" {
                why "The transport model retains unassigned command identifiers for forward-compatible error reporting while generation remains limited to supported commands."
                spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#usb-commands"
                given "an unassigned seven-bit CTAPHID command identifier" {
                    let wire = 0xaa;
                }
                when "the command is decoded and encoded again" {
                    let command = CtapHidCommand::from_wire(wire);
                    let encoded = command.wire_value();
                }
                then "the identifier remains available for forward-compatible framing" {
                    assert_eq!(command, CtapHidCommand::Unknown(0x2a));
                    assert_eq!(encoded, wire);
                }
            }
        }
    }
}
