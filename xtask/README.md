# xtask

A development CLI for building applications, running tests, generating reports, creating distribution packages, and updating the shared version. Run `cargo xtask <command>` from the repository root.

## Common commands

| Goal | Command |
| --- | --- |
| Build applications for the current OS | `cargo xtask build --locked` |
| Build release binaries | `cargo xtask build --locked --release` |
| Test shared packages and generate an HTML report | `cargo xtask test-with-report` |
| Run Cargo tests and the current OS's system tests | `cargo xtask host-tests-with-report` |
| Preview the documentation website | `cargo xtask docs dev` |
| Update the shared version | `cargo xtask set-version 0.6.4` |
| Advance to the next patch version | `cargo xtask bump-version` |

Use help to list commands and inspect their options:

```sh
cargo xtask --help
cargo xtask build --help
cargo xtask linux-system-test --help
cargo xtask dist --help
```

The `dist`, test, and report commands also have Cargo aliases. For example, `cargo dist` is equivalent to `cargo xtask dist`, and `cargo windows-system-test` is equivalent to `cargo xtask windows-system-test`. The examples below use `cargo xtask` consistently.

## Build applications

```sh
cargo xtask build --locked
cargo xtask build --locked --release
```

| Host OS | Applications built |
| --- | --- |
| Windows | Windows application |
| macOS | macOS application |
| Linux, including WSL | Both the Linux application and the WSL proxy |

Output goes to `target/debug/`, or `target/release/` with `--release`. Use `--locked` to prevent changes to Cargo.lock. Required shared libraries are built as dependencies. This command does not run tests.

Plain `cargo build` targets only the shared packages. To select a package explicitly, use `cargo build -p <package>`.

## Run tests

Use Cargo directly to run tests without generating a report:

```sh
cargo test
cargo test -p keyringd-core
cargo test -p keyringd-windows
```

Without arguments, `cargo test` targets the shared packages. To run an OS-specific package's unit tests, select it with `-p` on the corresponding OS.

### Cargo tests with an HTML report

```sh
cargo xtask test-with-report
cargo xtask test-with-report -- -p keyringd-core
cargo xtask test-with-report -- -p keyringd-core test_name
```

Arguments after `--` are forwarded to `cargo test`. The HTML report is written to `target/test-reports/index.html`.

### Product coverage (stable Rust)

Install the LLVM tools matching the active Rust toolchain, then add `--coverage` to a test command:

```sh
rustup component add llvm-tools-preview
cargo xtask test-with-report --coverage -- -p keyringd-core
cargo xtask linux-system-test --coverage --distribution ubuntu
cargo xtask windows-system-test --coverage
cargo xtask macos-system-test --coverage
cargo xtask wsl-system-test --coverage --distribution ubuntu
cargo xtask host-tests-with-report --coverage
```

xtask instruments workspace crates with Rust's LLVM coverage support and relocated counters, runs the existing test command, and exports coverage even if tests fail. Cargo's workspace compiler wrapper leaves external dependencies and build tooling uninstrumented. Continuous profile saving preserves application coverage when system tests terminate a fixture. No nightly toolchain or separate cargo-llvm-cov installation is needed.

Coverage preserves Cargo's debug-info settings. CI uses `line-tables-only` for development and test builds to retain source locations without full type and variable debug information.

Each run uses fresh `target/coverage-build/<suite>-<os>/` binaries and `target/coverage/<suite>-<os>/` profiles. The latter directory contains `lcov.info` and `metadata.json`, the portable aggregation artifacts. Measurement only exports these artifacts; use `coverage-report` below to generate HTML. Suites are `ut`, `system`, `system-ubuntu`, `system-fedora`, or `host-tests`. The UT command covers the selected Cargo UT/IT and specification tests together.

Only product crates' `src/` files contribute to the report. Runners, xtask, dependencies, separate test files, `test_support`, `ui_test_support`, and UI fixture entry points are excluded. Testing these components still works normally.

For packaged applications built on a different job, use `cargo xtask coverage-build -- <cargo build arguments>`. It writes instrumented binaries under `target/coverage-build/packaged/`. Supply their original binaries under `target/coverage-inputs/` on the test host so xtask can read their coverage mappings. WSL tests forward the profile path through WSL interop and the socket-activated user service.

To merge saved artifacts and render one combined HTML report, install [grcov](https://github.com/mozilla/grcov) on the report-generation host:

```sh
cargo install grcov --version 0.10.8 --locked
cargo xtask coverage-report downloaded-artifacts target/coverage-reports
cargo xtask coverage-report downloaded-artifacts target/coverage-reports --require ut/linux --require system-ubuntu/linux
```

xtask normalizes Windows/Linux/macOS source paths and Rust function names when exporting artifacts, and verifies the source revision and content hashes before aggregation. Required environments must all be present. grcov merges execution counts by source location and generates one combined report with coverage colors, bars, directory navigation, and source views. The output uses grcov's standard layout: `html/index.html` and the merged `lcov` trace. Percentages are never averaged. Per-OS and per-suite data remain in the Actions artifacts; generate a focused report from the selected artifacts only when needed. Only the report-generation host needs grcov; individual test hosts use the LLVM tools already required for measurement. To render a local measurement, run `cargo xtask coverage-report target/coverage/ut-linux target/coverage-reports` (replace `ut-linux` with the measured suite and OS).

The **Coverage and test reports** workflow runs UT/IT and all Linux, Windows, macOS, and WSL system-test jobs for the same commit, then publishes their reports and aggregate coverage to Surge. Start it manually with `workflow_dispatch`; pushes do not start this workflow. Individual workflows retain their triggers and optional `coverage` input, and only upload result artifacts. Report generation and publication belong to the final jobs of the coverage-reports workflow; publication remains restricted to `master` and uses the repository's `SURGE_TOKEN` secret.

This workflow is dedicated to coverage measurement. With `coverage: true`, the reusable CI workflow runs instrumented tests directly, without preceding lint, normal application builds, or package-validation builds. The ordinary CI path (`coverage: false`) retains those checks. System tests still build the instrumented applications and packages needed to execute their scenarios. Cargo builds xtask itself to launch the measurement commands.

### OS-specific tests

| Command | Host environment | Main options |
| --- | --- | --- |
| `linux-system-test` | Linux | `--distribution ubuntu\|fedora`, `--package PATH`, `--filter TEXT` |
| `macos-system-test` | macOS | `--filter TEXT`, `--case-timeout-seconds N` |
| `windows-system-test` | Windows | `--filter TEXT` |
| `wsl-system-test` | Windows with WSL | `--distribution ubuntu\|fedora`, `--package-directory PATH`, `--filter TEXT` |

Append `-with-report` to any of these command names to generate an HTML report after testing:

```sh
cargo xtask linux-system-test-with-report --distribution fedora --filter profile_management
cargo xtask macos-system-test-with-report --case-timeout-seconds 240
cargo xtask windows-system-test-with-report
cargo xtask wsl-system-test-with-report --distribution ubuntu --package-directory target/wsl-packages
```

The default distribution is `ubuntu`. The default macOS timeout is 180 seconds per case. Linux system tests build a test package automatically when `--package` is omitted. WSL tests use `target/wsl-packages/` as the default package directory.

Linux, macOS, and Windows runners build one `scenarios` executable from [system-tests](../tests/keyringd-system-tests/README.md). Shared cases are defined once; OS-specific cases and drivers are selected with `#[cfg]`. WSL package tests remain a separate crate. See the [development guide](../docs/internal/DEVELOPMENT.md) and the linked test runner documentation for required tools and environment setup.

### Run Cargo tests and host system tests together

```sh
cargo xtask host-tests-with-report
cargo xtask host-tests-with-report --filter profile_management -- -p keyringd-core
```

This combines Cargo test results and the current host’s system test results in one report. `--filter` applies to the OS-specific tests; arguments after `--` apply to Cargo tests. `--distribution` and `--package` apply only to Linux system tests.

Without arguments, the Cargo stage targets the shared packages. This command does not automatically include OS-specific packages' unit tests or WSL system tests on Windows.

### View reports or generate a site from saved results

| Report command | Default HTML output |
| --- | --- |
| `test-with-report` / `host-tests-with-report` | `target/test-reports/index.html` |
| `linux-system-test-with-report` | `target/test-reports/linux-system/<distribution>/index.html` |
| `macos-system-test-with-report` | `target/test-reports/macos-system/index.html` |
| `windows-system-test-with-report` | `target/test-reports/windows-system/index.html` |
| `wsl-system-test-with-report` | `target/test-reports/wsl-system/<distribution>/index.html` |

For individual OS-specific test commands, `--filter` records results under a `focused/` subdirectory. Set `KEYRINGD_TEST_REPORT_DIR` to override an individual OS-specific test command's result directory; relative paths resolve from the workspace root, and the directory must be below `target/`. Every execution resets its result directory before running. Commands that generate HTML attempt rendering even if tests fail.

To generate a complete report site from saved results, specify the input and output directories:

```sh
cargo xtask test-report results reports
cargo xtask test-report results reports --source-base-url https://github.com/OWNER/REPO/blob/COMMIT
```

`--source-base-url` supplies links to the tested source revision. See [testkit](../tests/keyringd-testkit/README.md) for the input data layout.

## Preview or build the documentation website

JDK 21 or newer is required. These commands use the Maven Wrapper in `website/`:

```sh
cargo xtask docs dev
cargo xtask docs dev --port 8081
cargo xtask docs build
cargo xtask docs build --base-path /keyringd
```

`dev` starts a development server on port 8080 by default. `build` generates static HTML in `website/target/roq/`. The default `--base-path` is `/`. See [website](../website/README.adoc) for site content and configuration.

## Create distribution packages

`dist` creates packages from an existing binary. Build the application first, for example with `cargo xtask build --locked --release`.

| `--target` | Supported `--format` values |
| --- | --- |
| `linux` | `deb`, `rpm` |
| `wsl` | `deb`, `rpm` |
| `windows` | `msi`, `runtime` |
| `macos` | `app` |

The default `--output-dir` is `dist/`. Run packaging on the corresponding OS with its required packaging tools installed.

Debian packages are assembled directly with `dpkg-deb`; no changelog is maintained. Install `binutils` for `strip`, and `dpkg-dev` for Linux shared-library dependency detection with `dpkg-shlibdeps`. The package Maintainer is `Keyringd maintainers <info@keyringd.com>`.

Create Linux deb or rpm packages:

```sh
cargo xtask dist --target linux --format deb target/release/keyringd --output-dir dist
cargo xtask dist --target linux --format rpm target/release/keyringd --output-dir dist
```

Create a Windows MSI or collect the Windows runtime:

```powershell
cargo xtask dist --target windows --format msi target/release/keyringd.exe --output-dir dist
cargo xtask dist --target windows --format runtime target/release/keyringd.exe --output-dir dist
```

The runtime is written to `dist/windows-runtime/`. Create WSL packages on Linux by combining the proxy with a Windows runtime. For this example, copy the runtime generated on Windows into `windows-runtime/` first:

```sh
cargo xtask dist --target wsl --format deb target/release/keyringd-wsl-proxy --windows-exe windows-runtime/keyringd.exe --windows-runtime-dir windows-runtime --output-dir dist
```

Build a macOS development app or a notarized release archive with xtask.

```sh
cargo xtask dist --target macos --format app --ad-hoc target/debug/keyringd-macos --output-dir dist/macos
cargo xtask dist --target macos --format app target/release/keyringd-macos --output-dir dist
```

`app --ad-hoc` creates `keyringd.app` for local development. Move the bundle to Applications and open it to register login startup. The app requires macOS 13 or later.

For release, `app` signs the application with a Developer ID Application identity, submits an archive for notarization, staples and validates the application ticket, then creates `keyringd-VERSION-ARCH.app.zip`. Release builds require the application identity, provisioning profile, and Apple notarization credentials. The archive contains the app with its stapled ticket.

When running a prebuilt xtask binary in an environment with a different repository mount point, use `dist --workspace /workspace` to specify the repository root visible there.

## Update the shared version

```sh
cargo xtask set-version 0.6.4
cargo xtask set-version 0.6.4-nightly
cargo xtask bump-version
```

This updates the root `[workspace.package].version` and Cargo.lock. Applications, libraries, development tools, and test packages all inherit this version. The command does not create a commit or publish a release.

`bump-version` reads the current workspace version, increments its patch number, removes prerelease and build suffixes, and prints the updated version.

Use the [Release workflow](../.github/workflows/release.yml) for releases and the [Nightly workflow](../.github/workflows/nightly.yml) for nightly builds.
