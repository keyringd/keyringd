use keyringd_testkit_macros::scenario;

scenario! {
    #[cfg(any())]
    case "excluded synchronous case" {
        verify "unavailable platform code stays uncompiled" {
            unavailable_platform_type::run();
        }
    }

    group "platform cases" {
        #[cfg(any())]
        case "excluded asynchronous examples" {
            async: tokio;
            ignored: "platform is absent";
            examples: ["one" => unavailable_example(), "two" => another_example()];
            verify "body and ignored metadata stay uncompiled" {
                unavailable_platform_type::run(example).await;
            }
        }
    }
}

fn main() {}
