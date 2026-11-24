use crate::ctap::Pin;
use crate::ctap::PinValidationError;

keyringd_testkit::scenario! {
        case "Normalize Text to UTF-8 NFC" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authnrClientPin-pin-composition"

            verify "Normalize Text to UTF-8 NFC" {
                assert_eq!(
                    Pin::from_text("e\u{301}123").unwrap().expose_bytes(),
                    "é123".as_bytes()
                );
                }

        }
        case "Validate CTAP PIN Configuration Requirements" {
            spec "https://fidoalliance.org/specs/fido-v2.2-ps-20250714/fido-client-to-authenticator-protocol-v2.2-ps-20250714.html#authnrClientPin-pin-composition"

            given "Prepare a boundary between the number of Unicode characters and the number of UTF-8 bytes, the trailing NUL" {
                let three_unicode_characters = "😀12";
                let four_unicode_characters = "😀123";
                let sixty_three_bytes = "a".repeat(63);
                let sixty_four_bytes = "a".repeat(64);
                let trailing_zero = "1234\0";
        }
        when "Build each string as a CTAP PIN" {
            let short_result = Pin::from_text(three_unicode_characters);
            let four_character_result = Pin::from_text(four_unicode_characters);
            let maximum_result = Pin::from_text(&sixty_three_bytes);
            let too_long_result = Pin::from_text(&sixty_four_bytes);
            let zero_result = Pin::from_text(trailing_zero);
        }
        then "Contains 4 Unicode characters and 63 UTF-8 bytes, forcing each boundary accurately" {
            assert!(matches!(
                short_result,
                Err(PinValidationError::TooShort)
            ));
            assert_eq!(
                four_character_result.unwrap().expose_bytes(),
                four_unicode_characters.as_bytes()
            );
            assert_eq!(
                maximum_result.unwrap().expose_bytes(),
                sixty_three_bytes.as_bytes()
            );
            assert!(matches!(
                too_long_result,
                Err(PinValidationError::TooLong)
            ));
            assert!(matches!(
                zero_result,
                Err(PinValidationError::TrailingZero)
            ));
        }

        }
}
