#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CtapVersion {
    Fido20,
    Fido21Preview,
    Fido21,
    Fido23,
}

impl CtapVersion {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fido20 => "FIDO_2_0",
            Self::Fido21Preview => "FIDO_2_1_PRE",
            Self::Fido21 => "FIDO_2_1",
            Self::Fido23 => "FIDO_2_3",
        }
    }

    pub fn from_version_identifier(value: &str) -> Option<Self> {
        match value {
            "FIDO_2_0" => Some(Self::Fido20),
            "FIDO_2_1_PRE" => Some(Self::Fido21Preview),
            "FIDO_2_1" => Some(Self::Fido21),
            "FIDO_2_3" => Some(Self::Fido23),
            _ => None,
        }
    }

    pub fn highest_supported<'a>(versions: impl IntoIterator<Item = &'a str>) -> Option<Self> {
        versions
            .into_iter()
            .filter_map(Self::from_version_identifier)
            .max()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    keyringd_testkit::scenario! {
        group "CtapVersion::highest_supported" {
            case "Selects the highest supported version independent of advertisement order" {
                verify "resolve the advertised version list to the same maximum in either order" {
                    assert_eq!(
                        CtapVersion::highest_supported(["FIDO_2_0", "FIDO_2_1_PRE", "U2F_V2"]),
                        Some(CtapVersion::Fido21Preview)
                    );
                    assert_eq!(
                        CtapVersion::highest_supported(["FIDO_2_3", "FIDO_2_1"]),
                        Some(CtapVersion::Fido23)
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "CtapVersion::from_version_identifier" {
            case "Recognizes each supported ctap dialect" {
                verify "map each supported FIDO version string to its matching CTAP dialect" {
                    for (identifier, expected) in [
                        ("FIDO_2_0", CtapVersion::Fido20),
                        ("FIDO_2_1_PRE", CtapVersion::Fido21Preview),
                        ("FIDO_2_1", CtapVersion::Fido21),
                        ("FIDO_2_3", CtapVersion::Fido23),
                    ] {
                        assert_eq!(
                            CtapVersion::from_version_identifier(identifier),
                            Some(expected)
                        );
                        assert_eq!(expected.as_str(), identifier);
                    }
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "CtapVersion::from_version_identifier" {
            case "Ctap 22 has no version identifier" {
                verify "reject FIDO_2_2 as a version identifier" {
                    assert_eq!(CtapVersion::from_version_identifier("FIDO_2_2"), None);
                    assert_eq!(
                        CtapVersion::highest_supported(["FIDO_2_1", "FIDO_2_2"]),
                        Some(CtapVersion::Fido21)
                    );
                }
            }
        }
    }
}
