use keyringd_testkit_macros::scenario;

scenario! {
    case "rejects a timeout on a synchronous scenario" {
        timeout: std::time::Duration::from_secs(1);

        given "some state" {}
        when "an operation runs" {}
        then "a result is checked" {}

    }
}

fn main() {}
