use std::error::Error;
use std::fmt;

/// Formats an error together with every source in its causal chain.
///
/// Logging boundaries can wrap an error in `ErrorChain` to retain every lower-level cause.
/// UI boundaries use `ApplicationError::user_message` or an operation-specific message instead
/// of displaying diagnostic strings.
pub struct ErrorChain<'a> {
    error: &'a (dyn Error + 'static),
}

impl<'a> ErrorChain<'a> {
    pub fn new(error: &'a (dyn Error + 'static)) -> Self {
        Self { error }
    }
}

impl fmt::Display for ErrorChain<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.error)?;
        let mut source = self.error.source();
        while let Some(error) = source {
            write!(formatter, ": {error}")?;
            source = error.source();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::util::test_support::{InnerError, OuterError, RootError};

    keyringd_testkit::scenario! {
        group "ErrorChain::fmt" {
            case "Formats every error in the source chain" {
                given "an outer error whose source chain contains an inner failure and its root cause" {
                    let error = OuterError {
                        source: InnerError { source: RootError },
                    };
                }
                when "the complete error chain is formatted for diagnostics" {
                    let formatted = ErrorChain::new(&error).to_string();
                }
                then "the message contains every cause from the outer failure through the root cause" {
                    assert_eq!(formatted, "outer failure: inner failure: root cause");
                }
            }
        }
    }
}
