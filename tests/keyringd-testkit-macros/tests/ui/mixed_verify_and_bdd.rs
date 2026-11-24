use keyringd_testkit_macros::scenario;

scenario! {
    case "rejects mixed verification styles" {

        verify "an invariant" {}
        given "state" {}
        when "action" {}
        then "result" {}

    }
}

fn main() {}
