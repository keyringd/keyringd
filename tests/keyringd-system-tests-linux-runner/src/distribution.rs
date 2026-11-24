use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Distribution {
    #[default]
    Ubuntu,
    Fedora,
}

impl Distribution {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Ubuntu => "ubuntu",
            Self::Fedora => "fedora",
        }
    }

    pub(crate) fn release(self) -> &'static str {
        match self {
            Self::Ubuntu => "resolute",
            Self::Fedora => "44",
        }
    }

    pub(crate) fn image_file_name(self, definition_hash: &str) -> String {
        format!(
            "{}-{}-x86_64-{definition_hash}.raw",
            self.id(),
            self.release()
        )
    }
}

impl fmt::Display for Distribution {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.id())
    }
}

impl FromStr for Distribution {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "ubuntu" => Ok(Self::Ubuntu),
            "fedora" => Ok(Self::Fedora),
            _ => Err(format!(
                "unsupported Linux system test distribution {value:?}; expected ubuntu or fedora"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Distribution;

    keyringd_testkit::scenario! {
        group "Distribution::from_str" {
            case "Parses supported distributions" {
                verify "parse Ubuntu and Fedora as the supported test distributions" {
                    assert_eq!("ubuntu".parse(), Ok(Distribution::Ubuntu));
                    assert_eq!("fedora".parse(), Ok(Distribution::Fedora));
                }
            }
        }
    }
}
