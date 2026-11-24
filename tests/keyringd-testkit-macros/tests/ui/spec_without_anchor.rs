use keyringd_testkit_macros::scenario;

scenario! {
    case "rejects a specification URL without an anchor" {
        spec "https://example.com/specification"

        verify "the specification reference is checked at compile time" {}
    }
}

fn main() {}
