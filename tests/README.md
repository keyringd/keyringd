# Tests

Place test authenticators, record of results/trails, generate reports, system tests and runners for each OS. Common-supported Crate is used in both unit and system tests.

See [Development Guide](../docs/internal/DEVELOPMENT.md) for the execution commands, the required environment, and how to verify the destination.

## Responsibilities and Placement of Tests

Product Crate tests are placed with the following policy.

| Kind | Contents to be verified | Arrangement |
| --- | --- | --- |
| UT | Combinations of type and processing own rules, boundary values, invariants, and settings | `#[cfg(test)] mod tests` in the implementation file |
| IT (Integrated Test) | Operation, change state, and linkage of side effects through multiple types/modules | Responsible for each Crate's `src/tests/` (e.g. `src/tests/application/`) |
| External specification conformance | Behavior defined by the relevant RFC, CTAP specification, or other external standard | Each crate’s `src/tests/spec/<specification>/` |
| System Test | Product-wide behavior including OS services, UI, and actual SSH connections | Dedicated tests in this directory Crate |

Even if the linked test uses only the public API, it will be unified to `src/tests/`. If you need to build a non-disclosure model or set state for testing, it will be handled at the same location and will not be distributed by the use of public and private APIs.

UT builds the model directly and does not launch Application setup or external services to check the input constraints of the model. Verify the combination of conditions with UT, and confirm that the model and the caller, save, and signature are linked correctly. Independent behavior separates the case, and the difference in input of the same rule is grouped into table format validation and multiple assertions.

For unit tests, put scenarios in a `group` named after the method or function they test, such as `Profile::validate` or `user_entity_at`. Name each case after the behavior or condition it verifies. The report then identifies both the function under test and the behavior being checked. Group integration tests by the operations that work together or the responsibility they cover.

When verifying constant values or enum identification values without using a function, use the target constant name/type name as the group name. We do not add methods just to give a group name.

UT is written in `keyringd_testkit::scenario!`, which can record the results in the report. Both the synchronous and asynchronous tests are in this notation, and the asynchronous case is `async: tokio;` and the expiration date is specified. Avoid leaking from the report by defining only with the normal `#[test]` or `#[tokio::test]`.

Entry points for subprocess tests, such as tests that trigger a panic, may use `#[test]` and `#[ignore]`. The parent scenario launches the subprocess explicitly and verifies and records its exit status and output.

`spec` is not used as an IT alias, but puts tests to validate compliance with external specifications. Whether or not the specifications are compliant and the granularity of UT/IT is on different axes.

The test modules are read from `mod.rs` to the regular `mod`, and the shared auxiliary type is placed in `src/test_support/`. Unify the name of the Cargo Feature that enables the test code to be `test-support`.

## Common test support

| Crate | Role |
| --- | --- |
| [test-authenticator](keyringd-test-authenticator/README.md) | Authenticator model responding to CTAP requests in Process |
| [test-usbip](keyringd-test-usbip/README.md) | Virtual AuthenticatorServer, USB/IP, Build and Control of Hybrid Tunnel |
| [testkit](keyringd-testkit/README.md) | Common Fixture, Recording Test Results/Trails, Generating HTML Reports |
| [testkit-macros](keyringd-testkit-macros/README.md) | Macros for Scenario and Evidence |

## system test

3OS cases are aggregated into [system-tests](keyringd-system-tests/README.md). The common case is defined only once, and the OS-specific case is limited to `#[cfg]`. Each OS runner builds and runs one `scenarios` executable file with the same crate, and uses another OS driver within the same crate.

| OS | Test | Runner |
| --- | --- | --- |
| Linux | [System tests](keyringd-system-tests/README.md#linux) | [Runner](keyringd-system-tests-linux-runner/README.md) |
| macOS | [System tests](keyringd-system-tests/README.md#macos) | [Runner](keyringd-system-tests-macos-runner/README.md) |
| Windows | [System tests](keyringd-system-tests/README.md#windows) | [Runner](keyringd-system-tests-windows-runner/README.md) |
| WSL2 | [System tests](keyringd-wsl-system-tests/README.md) | [Windows Host Common Runner] |

The validation range and fixture are described in the README of each test. Runners are in charge of building, running environments, auxiliary servers, and running and cleaning up the case.

## Test report

**Coverage and Test Reports** Workflows are UT/IT specification testing of the same commit and system for Linux (Ubuntu/Fedora), Windows, macOS, WSL (Ubuntu/Fedora). Run the tests together, and finally gather coverage and publish it to your Surge. Start only by manual operation by `workflow_dispatch`, not with push. Individual workflows can still be launched, but the results are saved to Artifact, and the publication to the site will be summarized in this batch execution.

Integrated workflows are dedicated to coverage measurement. UT/IT directly builds and runs tested tests and does not run the normal lint, app build, and package verification first. These verifications continue in the normal CI. Build the app packages required to run System Test with measurements. A build of the XTASK itself, which is the entry point, is required.

[The coverage report](https://keyringd-test-reports.surge.sh/coverage/html/index.html) generates and publishes only one total of the total environment. The measurement data for each OS and execution classification are stored in the Actions Artifacts, and reports are generated from the target Artifact when needed. Use `grcov` for aggregation and HTML generation, and display the color coding of the list and the execution status of the bar and source line. Aggregation sums up the execution results of the source line, and if one of the tests passes the same line, it is already executed. Do not expose the combined results when the results of different commit sources or the required environment are missing. Test runners and test support codes are excluded from the aggregate.

Locally, add `--coverage` to existing xtask test commands. Save measurement data using Stable Rust's `llvm-tools-preview` and generate coverage HTML with GRCOV with `cargo xtask coverage-report`. The HTML of the test results is generated by an existing test command. See [XTASK README](../xtask/README.md#product-coverage-stable-rust) for save destinations and aggregation commands.

**Coverage and Test Reports** publish the results of all environments on one site. The link destination is the result of the batch execution that was successfully published at the end, and is not updated when the individual workflow is executed. Test Report shows execution results and trails, and Requirements Report indicates specifications and test responses. Requirements Report is generated from the results of UT/IT and specification tests.

| Kind | Environment | Test Report | Requirements Report |
| --- | --- | --- | --- |
| CI (UT/IT/Specification Test) | Linux | [Test result](https://keyringd-test-reports.surge.sh/ut/linux/index.html) | [Specifications compatible](https://keyringd-test-reports.surge.sh/ut/linux/requirements.html) |
| CI (UT/IT/Specification Test) | macOS | [Test result](https://keyringd-test-reports.surge.sh/ut/macos/index.html) | [Specifications compatible](https://keyringd-test-reports.surge.sh/ut/macos/requirements.html) |
| CI (UT/IT/Specification Test) | Windows | [Test result](https://keyringd-test-reports.surge.sh/ut/windows/index.html) | [Specifications compatible](https://keyringd-test-reports.surge.sh/ut/windows/requirements.html) |
| System Test | Linux / Ubuntu | [Test result](https://keyringd-test-reports.surge.sh/system/linux/ubuntu/index.html) | — |
| System Test | Linux / Fedora | [Test result](https://keyringd-test-reports.surge.sh/system/linux/fedora/index.html) | — |
| System Test | macOS | [Test result](https://keyringd-test-reports.surge.sh/system/macos/index.html) | — |
| System Test | Windows | [Test result](https://keyringd-test-reports.surge.sh/system/windows/index.html) | — |
| System Test | WSL2 / Ubuntu | [Test result](https://keyringd-test-reports.surge.sh/system/wsl/ubuntu/index.html) | — |
| System Test | WSL2 / Fedora | [Test result](https://keyringd-test-reports.surge.sh/system/wsl/fedora/index.html) | — |
