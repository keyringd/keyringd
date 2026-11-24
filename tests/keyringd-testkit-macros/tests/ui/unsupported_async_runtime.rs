use keyringd_testkit_macros::scenario;

scenario! {
    case "rejects an unsupported asynchronous runtime" {
        async: async_std;

        given "some state" {}
        when "an operation runs" {}
        then "a result is checked" {}

    }
}

fn main() {}
