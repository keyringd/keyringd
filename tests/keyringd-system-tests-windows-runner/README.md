# keyringd-system-tests-windows-runner

The Windows host runner executes Windows UI tests and [WSL system tests](../keyringd-wsl-system-tests/README.md) as independent suites. They share the run lock, authenticator and tunnel, console control, case selection, and result reconciliation. The Windows suite also configures Windows OpenSSH and Security Automation.

The Windows suite builds one `scenarios` executable for the [system tests](../keyringd-system-tests/README.md#windows) and filters and reconciles results for shared and Windows-specific cases. The crate keeps UI operation types under `src/application/features/<feature>/windows/` and test environment ownership under `src/environment/windows/`. The WSL suite uses a separate crate.

## Execution

Prepare Windows MSVC Toolchain, Desktop, JDK, USBIP-Win2, Windows OpenSSH Client/Server, and Git for Windows. [Tests Readme](../keyringd-system-tests/README.md) describes the introduction procedure. The versions of the Git included SSH are also recorded in the report execution environment.

```powershell
cargo windows-system-test-with-report
cargo windows-system-test-with-report --filter validation
```

No HTML is `cargo windows-system-test`. Fails on Windows other than Windows. Both normal and administrator can be started, and Case and Fixture inherit Runner's users and permissions.

CNG, WebAuthn PIN, or USB Cancel to check and operate the OS screen In Case, the dedicated Windows Security Automation Process is promoted once and shared among cases. Auxiliary PROCESS also stops when the Runner ends.

## Environment and execution management

| Portion | Duty |
| --- | --- |
| `build` | Get Fixture, Scenario, Automation Cargo Artifact |
| `environment` | Workspace/Cache, Run Lock, OpenSSH, Server, Console, Automation |
| `execution` | Case selection, child process settings, run, check results |

Choose OpenSSH from `%SystemRoot%\System32\OpenSSH`. Java can be resolved from `JAVA_HOME` or PATH, and `usbip.exe` can be resolved from PATH or `%ProgramFiles%\USBip`. The Authenticator JAR, Test CA, Empty Server, and Loopback Hybrid Tunnel are prepared with [Test-USBIP](../keyringd-test-usbip/README.md).

The UI execution of the same host is serialized with a Lock. The per-case Device/Attachment is collected on the Tests side, and the shared Server and Automation are recovered by the Runner. Get the executable from Cargo JSON and do not guess the exe name in the Build Directory.

A dedicated Windows Job supervises each case and cleans up descendant processes on exit, crash, or after 300 seconds. The runner saves logs and checkpoints under `cases/<id>/` in the report and records a failed result if a crash leaves no result JSON. Existing skip conditions apply.

The key to Software CNG is to record the deleted Intent before it was created on `device-keys/` under Case. Even after the fixture stops, the GUI will be retrieved with the cleanup command that does not start. Restart the normal Restart/Profile Erase the ledger, and try to collect the Fixture DROP and the case supervisor. A CLEANUP failure logs a case as a failure and does not proceed to a subsequent case and aborts the RUN. Leave uncollected Intents.

## Results and Verification

The results are `target/test-reports/windows-system/` and the filter run is `focused/`. Initialization of Windows results keeps the normal test results that coexist. Match case selection and JSON to detect missing, duplicate, unexpected, and fail. The HTML generation is done by XTASK.

```sh
cargo test -p keyringd-system-tests-windows-runner
```

Testing usually checks case selection, REPORT initialization, and so on. Verification of Windows API, OS Dialog, USB/IP Driver, and actual SSH is done with Windows System Test.

## WSL system testing

Prepare MSVC Toolchain, Desktop, JDK, USBIP-WIN2, and WSL2 for Windows Host, which does not install Keyringd for Windows. Windows OpenSSH is not required.

Prepare a DEB/RPM with the Runtime of Windows Companion with `test-support` and a DEB/RPM containing the Release/Musl version of Proxy in `target/wsl-packages`. CI artifacts `wsl-debian-package` or `wsl-rpm-package` are also available. Package generation uses `cargo dist --target wsl --format deb|rpm` in the same Linux environment as the release and does not build within WSL.

```powershell
cargo wsl-system-test-with-report --distribution ubuntu
cargo wsl-system-test-with-report --distribution fedora
```

The UBUNTU-26.04 or FedoraLinux-44 selected by RUNNER's `WslTestbed`, the clean dedicated `KeyringdE2EUbuntu` or `KeyringdE2EFedora` Download and install. It introduces only execution dependencies such as OpenSSH and Python, and does not introduce Rust or any tools for package generation. User's Distribution is not duplicated, and it fails without overwriting if there is an existing dedicated Distribution. Regardless of success or failure, check the destination Directory and Marker, and delete the dedicated Distribution that you have created.

`wsl-system-test` has no HTML, `wsl-system-test-with-report` has HTML. Select targets with `--distribution ubuntu|fedora`, `--package-directory PATH`, and `--filter TEXT`. The package-directory default is `target/wsl-packages`. Put only one package for each format.

WSL Suite installs the package and passes the installed Companion Windows Path to Scenario. Proxy and Companion use the default packages and do not replace them with BINARY for development. Companion comes with `test-support` and uses the STORE/SOFTWARE key/control endpoint for testing. Normally, it will be aborted if the Agent is running. Collect Proxy, SSHD, Fixture, and USB Attachment when the case ends.

Results go to `target/test-reports/wsl-system/<distribution>/`, with filtered runs under `focused/`. The dedicated [system-tests-wsl.yml](../../.github/workflows/system-tests-wsl.yml) workflow builds the companion and proxy and produces deb and rpm packages on Linux. Ubuntu and Fedora test the resulting packages in parallel on separate Windows runners. Tests on the same host run serially because they share the UI, agent pipe, and USB/IP. The workflow publishes results from both distributions together in a report separate from the Windows suite.
