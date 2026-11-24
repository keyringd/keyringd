# keyringd-system-tests-macos-runner

Run the product UI Fixture and [System test](../keyringd-system-tests/README.md#macos) successively on macOS Desktop. Owns a fixture build, authenticator/tunnel, case expiration, diagnostic process cleanup and result collation. CLI and HTML generation are placed in XTASK.

The runner builds one `scenarios` executable from `keyringd-system-tests` and filters and reconciles results for shared and macOS-specific cases. The crate keeps UI operation types under `src/application/features/<feature>/macos/` and test environment ownership under `src/environment/macos/`.

## Execution

Rust and macOS SDK, operational Desktop, JDK, OpenSSH SSH/SSHD/SSH-KEYGEN are required.

```sh
cargo macos-system-test-with-report
cargo macos-system-test-with-report --filter validation
cargo macos-system-test-with-report --case-timeout-seconds 240
```

No HTML is `cargo macos-system-test`. Calling Runner on a non-macOS macOS fails. The default case is 180 seconds. The suite output is `target/test-reports/macos-system/`, and the filter run is `focused/`.

## Running ownership

| Portion | Duty |
| --- | --- |
| `build` | Get a Fixture/Scenario Artifact from Cargo JSON and build a dedicated.app |
| `environment` | Temporary CA, Empty AuthenticatorServer, owned by Hybrid Tunnel |
| `execution` | Case list and selection, run sequential, deadlines, results/Check cleanup |

The dedicated Bundle uses the product name, Version, of the product Info.plist, and the executable file and the bundle ID are used for testing. Starting and stopping JARs, certificates, and servers will be delegated to [Test-USBIP](../keyringd-test-usbip/README.md).

Each case is run in a separate Process Group. If the expiration date is overdue, a diagnosis is made with a timed RPC, the Scenario, Fixture, SSH child process is terminated, and the Temporary State and the Residual Authenticator are recovered. Detects residual devices before and after the case, and if CleanUp cannot recover, the failure is recorded and stopped.

Use checkpoint to save the case where the process stopped before writing the JSON result as a fail. Treat the SKIP, Missing, Duplicate, and Unexpected results of the selected case as failures. Failures before the start of a case such as build or server startup are also recorded in the Runner result.

## Trails and Verification

Case diagnostics go to `cases/<number>/diagnostics/` and the server log to `authenticator-server/authenticator.log`. Save available app logs, profile JSON, accessibility trees, and screenshots, and record any capture failures. Record the selected SSH and Java paths and versions in `run-metadata.json`.

```sh
cargo test -p keyringd-system-tests-macos-runner --features test-support
```

`test-support` enables explicit support checking on Linux. Normally, tests do not start the GUI, and Linux can verify execution management, deadlines, failure results, and cleanup. Check [Scenario README](../keyringd-system-tests/README.md) for the warranty coverage of AppKit, Secure Enclave and Launchd.
