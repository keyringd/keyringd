use quote::format_ident;
use syn::{Error, LitStr, Result};

pub(crate) fn test_function_name(case_name: &LitStr) -> Result<syn::Ident> {
    let mut name = String::new();
    let mut separator = false;
    for character in case_name.value().chars() {
        if character.is_alphanumeric() || character == '_' {
            if separator && !name.is_empty() {
                name.push('_');
            }
            name.extend(character.to_lowercase());
            separator = false;
        } else {
            separator = true;
        }
    }
    if name.is_empty() {
        return Err(Error::new(
            case_name.span(),
            "case must contain a name that can be converted to a Rust identifier",
        ));
    }
    syn::parse_str::<syn::Ident>(&name)
        .map(|identifier| format_ident!("{identifier}", span = case_name.span()))
        .map_err(|_| {
            Error::new(
                case_name.span(),
                "case must start with a letter and be convertible to a Rust identifier",
            )
        })
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;

    use super::*;

    keyringd_testkit::scenario! {
        group "test_function_name" {
            case "Converts an english case name to a rust identifier" {
                verify "produce the expected Rust test identifier from the English caption" {
                    let case_name = LitStr::new(
                        "Rejects trailing-data after a known public key",
                        Span::call_site(),
                    );

                    let identifier = test_function_name(&case_name).unwrap();

                    assert_eq!(
                        identifier.to_string(),
                        "rejects_trailing_data_after_a_known_public_key"
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "test_function_name" {
            case "Converts a Unicode case name to a Rust identifier" {
                verify "preserve the accented character in the generated Rust identifier" {
                    let case_name = LitStr::new("Round-trips a café fixture", Span::call_site());

                    assert_eq!(
                        test_function_name(&case_name).unwrap().to_string(),
                        "round_trips_a_café_fixture"
                    );
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "test_function_name" {
            case "Rejects a case name that cannot become a rust identifier" {
                verify "return a diagnostic for a caption that yields no valid identifier" {
                    for value in ["", "123 invalid"] {
                        let case_name = LitStr::new(value, Span::call_site());

                        assert!(test_function_name(&case_name).is_err());
                    }
                }
            }
        }
    }
}
