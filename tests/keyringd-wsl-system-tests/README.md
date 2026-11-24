# keyringd-wsl-system-tests

This independent system suite tests the WSL proxy and bundled Windows companion on a host without a separate Keyringd for Windows installation. The [runner](../keyringd-system-tests-windows-runner/README.md) owns the execution environment, case selection, and helper servers.

```powershell
cargo wsl-system-test-with-report --distribution ubuntu
cargo wsl-system-test-with-report --distribution fedora
```

Install deb/rpm on a dedicated disposable distribution. After exiting the profile setting UI, start the package `keyringd.socket` and check that the Proxy Service and Windows Agent are stopped. The first SSH/SSH-ADD connection starts Service, and the included `/usr/libexec/keyringd/keyringd.exe bridge` automatically starts Windows Agent. It doesn't depend on development build artifacts or Windows PATH/MSI.

The creation, dependent introduction, and deletion of disposable Distributions is owned by `WslTestbed` of Windows Runner. Deb/rpm is pre-generated in the same Linux environment and XTask as the release. CI receives Artifact and verifies Ubuntu/Fedora in parallel with separate Windows Runners. The test itself is built and run for Windows, and does not use Rust or build tools within WSL.

In addition to enabling the package file systemd unit/socket, verify the real Linux OpenSSH identity enumeration, certificate, Device Key, Passkey authentication, reconnect, and reconnect after denying on Windows. The SSHD uses the Root owned Directory and Loopback Port exclusively for the case, and does not touch existing SSH settings.

Some WinUI SDKs have different case file names and Loader request names. In order to start Companion on Linux FileSystem directly, `Microsoft.UI.Xaml.Resources.19H1.dll`, `Microsoft.UI.Xaml.resources.common.dll`, and `en-US` are aligned with `cargo dist --target windows --format runtime`. Check the notation with installation verification, and check the lazy loading of the string resources in the theme/input field in the profile creation e2e.

The test package Companion is built with `test-support`. Enabling Control Endpoint/Case-only Store/Software Device Key using the same UI/Agent/Bridge as the product. USB connects to the product WebAuthn API with virtual USB/IP, and Hybrid connects to the actual Tunnel/Noise/CTAP with a virtual phone. In RDP, Windows WebAuthn transfers the USB to the client, so the USB case is used as a reason, and the HYBRID case is executed. Windows UI Driver and Page Object are shared, but Windows Scenario does not run. The actual TPM is guaranteed by a real-time Windows test.

Plain `cargo test` does not launch the scenarios. HTML reports are saved at `target/test-reports/wsl-system/<distribution>/index.html`, with filtered runs under `focused/`.

The Root User Manager for disposable distributions should be placed on a dedicated systemd slice per environment. It is a test environment setting that does not interfere with the existing root user manager on the cgroup hierarchy shared by WSL, and the unit of the package is used as is.

Confirm the registration of Windows integration (`WSLInterop`) as a prerequisite. Since the WSL Interpreter registration by systemd is skipped due to the empty `binfmt.d` in the initial image of Fedora, Runner registers the normal `/init` Interpreter. `cmd.exe` is also an environment preparation to prevent product testing from starting with the OS settings.
