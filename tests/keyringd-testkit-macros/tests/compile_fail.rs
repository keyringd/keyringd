keyringd_testkit::scenario! {
    group "compile fail" {
        case "Rejects invalid scenarios" {
            verify "rejects invalid scenarios" {
                let tests = trybuild::TestCases::new();
                tests.compile_fail("tests/ui/*.rs");
            }
        }
    }
}

keyringd_testkit::scenario! {
    group "compile fail" {
        case "Cfg excludes case bodies examples and ignored metadata" {
            verify "cfg excludes case bodies examples and ignored metadata" {
                let tests = trybuild::TestCases::new();
                tests.pass("tests/pass/cfg_cases.rs");
            }
        }
    }
}
