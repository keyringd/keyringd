use crate::ctap::ClientDataHash;

keyringd_testkit::scenario! {
        case "Hashing Client Data with SHA-256" {
            spec "https://www.w3.org/TR/webauthn-3/#dictionary-client-data"

            verify "Hashing Client Data with SHA-256" {
                assert_eq!(
                    ClientDataHash::hash(b"abc").as_bytes(),
                    &[
                        0xba, 0x78, 0x16, 0xbf, 0x8f, 0x01, 0xcf, 0xea, 0x41, 0x41, 0x40, 0xde, 0x5d, 0xae,
                        0x22, 0x23, 0xb0, 0x03, 0x61, 0xa3, 0x96, 0x17, 0x7a, 0x9c, 0xb4, 0x10, 0xff, 0x61,
                        0xf2, 0x00, 0x15, 0xad,
                    ]
                );
                }

        }
}
