use keyringd_testkit_macros::scenario;

scenario! {
    case "rejects empty examples" {
        examples: [];

        given "some state" {}
        when "an operation runs" {}
        then "a result is checked" {}

    }
}

fn main() {}
