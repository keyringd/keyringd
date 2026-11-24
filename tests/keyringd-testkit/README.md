# keyringd-testkit

Provides Scenario DSL, case execution, JSON results, trail, HTML Report and Runner common parts. Do not own the PROFILE, PassKeyService, or OS Testbed of the product.

Use `product_version::LINUX`/`MACOS`/`WINDOWS` for expected values for product versions such as About. Obtained at build time from cargo.toml of the product, and regenerated when changed. Test Crate's own `CARGO_PKG_VERSION` is not a product version.

## write a scenario

```rust
use keyringd_testkit::scenario;

scenario! {
    case "preserves a byte sequence" {
        given "a byte sequence" {
            let original = vec![0_u8, 1, 255];
        }
        when "the sequence is cloned" {
            let copied = original.clone();
        }
        then "all bytes remain in order" {
            assert_eq!(copied, original);
        }
    }
}
```

Given represents the premise, when is the target operation, then the observation result. Divide the procedure or the results you want to make independently into STEPs. There is no need to separate each assertion. For low-level invariant conditions, use `verify` instead of GIVEN/When/Then.

```rust
scenario! {
    group "u8::checked_add" {
        case "rejects overflow" {
            verify "adding one to the maximum returns none" {
                assert_eq!(u8::MAX.checked_add(1), None);
            }
        }
    }
}
```

Give the target method/function name to `group`. Do not mix `verify` and GIVEN/WHEN/THEN with the same case.

## DSL settings

| Syntax | Use |
| --- | --- |
| `why "..."` | Why a Case is Needed |
| `async: tokio;` | Asynchronous CASE (default to Tokio current-thread run) |
| `async: tokio(flavor = "multi_thread");` | Asynchronous case specifying the attribute argument of Tokio |
| `timeout: Duration::from_secs(5);` | Async Case Deadline |
| `examples: ["zero" => 0_u8, "max" => u8::MAX];` | Expand each value to an independent case. See `example` from STEP |
| `skip_if: condition => "reason";` | Skip with reason at run time |
| `ignored: "reason";` | Rust ignored test |

Expired and panics are recorded as FAIL, and Rust Test also fails. `skip_if` finishes normally as Rust Test, but SKIP in Report. Ignored case is explicitly selected in `cargo test ... -- --ignored`.

The [macros crate](../keyringd-testkit-macros/README.md) owns parsing, validation, and expansion. This crate owns execution at runtime.

## save the trail

Use `evidence_text!`, `evidence_log!`, `evidence_bytes!`, `evidence_file!`, and `evidence_image!` in STEP.

```rust
evidence_text!("actual value", format!("{actual:?}"));
evidence_image!("application window", screenshot_path);
```

The contents are not processed or masked. Do not bring in secret values and actual user data using artificial test state. The image does not replace the displayed assertion.

## Run and local HTML

```sh
cargo test-with-report
cargo test-with-report -- --package keyringd-core
cargo host-tests-with-report
```

Save parallel secure JSON for each case. `KEYRINGD_TEST_REPORT_DIR` is the recording destination, and the default is `target/test-reports`. If you rerun the same case, the record will be updated to Atomic. XTASK's OS-specific test command initializes the save destination before execution, regardless of whether HTML is generated or not. The overwrite destination is under Workspace's `target/`, and the relative paths are resolved from Workspace Root. Report Workflow also collates with the current source and generates HTML.

The HTML report shows cases, groups, sources, pass/fail/skip status, steps, duration, panics, code, and evidence, with search and filters. See the [development guide](../../docs/internal/DEVELOPMENT.md) for each suite’s commands and output paths.

Test Report and Requirements Report will make the body full width when printing the browser and reduce the text and margins. The table of contents, search, operation buttons, and test code are not printed, and results, procedures, and reasons for failure are left. In order to reflect the current search/filter and the open/close state of the trail, the print range and log display are selected on the screen before printing.
In print, show specification references by specification name and the URL anchor after `#`. Deduplicate identical references only. Print Requirements Report specification headings without underlining.
While the translation is prioritized for the display range, you can proceed from the top of the page to items that are hidden or hidden by filter. If you check the number of translations completed and then print, you can print the non-scrollable range in the selected language.

## Create a public site from the CI results

```text
results/
  ut/{linux,windows,macos}/
  system/linux/{ubuntu,fedora}/
  system/{macos,windows}/
```

```sh
cargo test-report results reports --source-base-url https://github.com/OWNER/REPO/blob/COMMIT
```

Place the CLI entrance to [XTASK](../../xtask/README.md) and call TestKit's `generate_report_site()`. Read the recorded JSON and reference trails and generate `reports/index.html` and HTML per environment. Don't delete records because of another Host's Source Path, and don't mix past sites into your input. Corrupted JSON and Missing Trails are errors.

Report generation also maps test IDs to the requirements in [requirements.yaml](../../docs/internal/requirements.yaml). It emits the Requirements Report and links only for unit and integration test reports under `ut/`. Reports under `system/` show system test results and evidence. The [GitHub Actions workflows](../../.github/workflows) handle CI uploads and publishing to Surge.

## API for Runner

| API | Use |
| --- | --- |
| `runner::CargoBuildArtifacts` | Get Executable for a specified Target from Cargo JSON |
| `runner::ScenarioSelection` | Analyze the LibTest list and represent a selected case |
| `runner::ScenarioResults` | Confirmation of missing/duplicated/unexpected/failed results of reading results and CASE |
| `runner::ReportRunMetadata` | Save Run ID, Start Time, Host, Git State |
| `runner::ToolExecutionMetadata` | Record the Path/Version of the actually selected Tool |
| `qr_code_reader::QrCodeReader` | Read QR with Pure Rust from PNG and more |
| `PxpAdvertisementRelay` | Send FFF9 ads to Product Agent in Extension |

`run-metadata.json` saves the environment at the time of recording. Tool diagnostics run on time and leave a reason for failure. Do not supplement by guessing the environment of past results by public generation.

It also provides a shared RPC type/Platform-independent search of macOS Fixture, a failed case from Runner, and a checkpoint. OS-specific Driver and Process-owned are placed on each OS Crate.

## Choosing a Support Crate

| Things necessary | owner |
| --- | --- |
| CTAP model in Process | [test-authenticator](../keyringd-test-authenticator/README.md) |
| Product PasskeyService, Software Device Key, Store Failure Injection | `keyringd-core::test_support` |
| Kotlin Server, HTTP control, USB/IP HID | [test-usbip](../keyringd-test-usbip/README.md) |
| OS environment, auxiliary process, case cleanup | Each OS Runner |
| User Scenario, Page Object | Each OS Tests |

```sh
cargo test -p keyringd-testkit
cargo test -p keyringd-testkit-macros
```
