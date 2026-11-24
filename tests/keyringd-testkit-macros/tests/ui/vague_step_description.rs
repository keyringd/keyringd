use keyringd_testkit_macros::scenario;

scenario! {
    case "rejects a vague step description" {
        given "state exists" {}
        when "Run the operation under test" {}
        then "a concrete result is returned" {}
    }
}

fn main() {}
