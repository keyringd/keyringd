use keyringd_testkit::{evidence_bytes, evidence_file, evidence_log, evidence_text, scenario};

scenario! {
    case "supports a single invariant verification" {
        spec "https://example.com/specification#single-invariant"
        verify "the invariant holds without artificial Given and When steps" {
            assert_eq!(2 + 2, 4);
        }
    }

    case "preserves values across asynchronous steps" {
        async: tokio;
        timeout: std::time::Duration::from_secs(1);

        given "a value is produced asynchronously" {
            let value = async { 41_u8 }.await;
        }
        when "the value is incremented after yielding" {
            tokio::task::yield_now().await;
            let result = value + 1;
        }
        then "the result remains available" {
            assert_eq!(result, 42);
        }
    }

    case "preserves the declared order of repeated actions and results" {
        given "a mutable counter starts at zero" {
            let mut counter = 0;
        }
        when "the first increment is applied" {
            counter += 1;
        }
        then "the first result is observable immediately" {
            assert_eq!(counter, 1);
        }
        when "the second increment is applied" {
            counter += 1;
        }
        then "the second result follows the second action" {
            assert_eq!(counter, 2);
        }
    }

    case "runs each parameterized example" {
        examples: [
            "zero" => 0_u8,
            "maximum" => u8::MAX,
        ];

        given "an example byte is available" {
            let encoded = [example];
        }
        when "the byte is read" {
            let decoded = encoded[0];
        }
        then "the original value is preserved" {
            assert_eq!(decoded, example);
        }
    }

    case "records a conditional skip" {
        skip_if: true => "demonstrates a skipped scenario";

        given "an unreachable setup" {}
        when "an unreachable operation runs" {}
        then "an unreachable result is checked" {}
    }

    case "records an ignored scenario without executing its body" {
        ignored: "requires an external test environment";

        verify "the ignored body is reserved for an explicit run" {
            assert!(true);
        }
    }

    case "records each evidence kind" {

        given "synthetic evidence values" {
            let file = std::path::Path::new("artifacts/scenario-features/test-artifact.txt");
            let report_directory = std::env::var_os("KEYRINGD_TEST_REPORT_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(keyringd_testkit::default_report_directory);
            let artifact = report_directory.join(file);
            std::fs::create_dir_all(artifact.parent().unwrap()).unwrap();
            std::fs::write(&artifact, b"synthetic file evidence").unwrap();
        }
        when "the values are attached" {
            evidence_text!("response", "synthetic response");
            evidence_log!("transport", "synthetic log");
            evidence_bytes!("wire", &[0x01, 0xab]);
            evidence_file!("artifact", file);
        }
        then "the scenario remains successful" {
            assert_eq!(std::fs::read(&artifact).unwrap(), b"synthetic file evidence");
        }
    }

    group "grouped operation" {
        case "records the first grouped case" {
            verify "the first case runs with its group metadata" {
                assert!(module_path!().ends_with("scenario_features"));
            }
        }
        group "nested operation" {
            case "records a case in a nested group" {
                verify "the nested case runs with its complete group path" {
                    assert_eq!(2 + 2, 4);
                }
            }
        }
    }
}

scenario! {
    group "grouped operation" {
        case "records the second grouped case" {
            verify "another macro invocation can share the group without a Rust module collision" {
                assert_eq!(2 + 2, 4);
            }
        }
    }
}
