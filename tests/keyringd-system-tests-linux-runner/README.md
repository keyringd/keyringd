# keyringd-system-tests-linux-runner

This host runner installs the Linux package in an isolated systemd-nspawn environment and runs the [system tests](../keyringd-system-tests/README.md#linux). It owns builds, environment resources, helper servers, execution, and cleanup. [xtask](../../xtask/README.md) provides the CLI and generates HTML.

The runner builds one `scenarios` executable from `keyringd-system-tests` and filters and reconciles results for shared and Linux-specific cases. The crate keeps UI operation types under `src/application/features/<feature>/linux/` and test environment ownership under `src/environment/linux/`.

## Prepare a host

On Ubuntu Host, prepare the following: Requires a Linux environment that can handle systemd-nspawn and USB/IP, sudo, Rust, and JDK.

```sh
sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  bluez bluez-test-tools build-essential default-jdk-headless dnf e2fsprogs git \
  kmod mkosi systemd-container libadwaita-1-dev libdbus-1-dev libgtk-4-dev \
  libtss2-dev libudev-dev linux-tools-generic openssl pkg-config rpm
```

Runner does not install host dependent. To validate to virtual Bluetooth, you need Kernel `hci_vhci` and `btvirt`:

```sh
sudo modprobe hci_vhci
test -e /dev/vhci
command -v btvirt
```

The CI prepares these with [Linux-System-Test-Host Action](../../.github/actions/linux-system-test-host/action.yml). If `hci_vhci` does not exist, it will notify and continue, and skip only BLE dependent cases. If a module exists but loads and checks for `/dev/vhci` fails, SETUP fails.

## Execution

```sh
cargo linux-system-test-with-report
cargo linux-system-test-with-report --distribution fedora
cargo linux-system-test-with-report --distribution ubuntu --filter profile_management
```

The default is Ubuntu 26.04, Fedora is 44. Select Automatic build package for Distribution. If you want to try an existing package:

```sh
cargo linux-system-test-with-report --distribution ubuntu --package /path/to/package.deb
cargo linux-system-test-with-report --distribution fedora --package /path/to/package.rpm
```

Run without HTML is `cargo linux-system-test`. Concurrent executions across CHECKOUT are also denied with Run Lock to share the same host's fixed resources.

## Testbed configuration

1. Prepare the product Binary, Scenario Executable, and AuthenticatorServer JAR on the non-privileged side.
2. Select MKOSI Image and run Testbed control from a single XTask boundary.
3. Prepare systemd-nspawn, software TPM, XVFB, User Session, USB/IP. Inside the container, run `dist --workspace /workspace` from the XTASK executable file that is started, and generate and install the package with the same process as the release.
4. Run Scenario and save the results and diagnostics.
5. Container, USB/IP Attachment, collect auxiliary resources.

The mkosi definition is [Testbed/MKOSI](testbed/mkosi). Raw Images are cached to `target/linux-system/images` and identified by Distribution Release, Architecture, and Hash of the definition. Each run uses volatile overlay. Bind Repository Root to `/workspace` with Writable and share the regular Artifact Path. Share Host Network and don't create dedicated Bridges or NATs.

Fixed dependencies put in image. The command in the container uses `systemd-run --machine`, and the termination uses `machinectl poweroff`. Specify `GSK_RENDERER=cairo` for XVFB. Disable the number of startup limits for the User Service in Testbed because it will reboot repeatedly.

Create as many cases of a case as you need. Runner's Privileged Broker can be used to set ownership of USB/IP and HiDRAW through UNIX Sockets (`KEYRINGD_E2E_USB_CONTROL`) dedicated to E2E users (Mode). 0600), do a dynamic bind to the container. When disconnected, unbind and USB/IP Detach are used, and when reconnecting to the same key maintains the saved credentials. Creating and deleting the authenticator is owned by the Common Fixture, and the Broker will recover the remaining connections even if the case process is terminated.

## Hybrid validation path

| Route | What to verify | Prerequisites/Limits |
| --- | --- | --- |
| PXP relay | Screen QR, Virtual Phone, Products Tunnel/Noise/CTAP, Enter Advertising | OPT-IN the agent's relay in the case. Bluez Scan does not pass |
| btvirt / BlueZ | Reached the product BtlePlug Scanner from a virtual BLE ad | Requires `hci_vhci` and `/dev/vhci`. Skip the target case if you are absent |
| Absence of the Adapter | Product Real Adapter Detection and USB Select UI | Temporarily hide the Bluetooth sysfs in the container |

The virtual phone uses a test TLS Tunnel for `cable.ua5v.com`. CA and PORT transfers are limited to Testbed. It does not guarantee the interoperability of all Tunnel identifiers selected by the real phone.

The `relay-hybrid-advertisements` command itself, which is scanned by another machine, is not covered by the current SCENARIO. Do not confuse direct transmission to PXP Extension with its Scanner validation.

## Results and implementation

Results are saved per distribution under `target/test-reports/linux-system/<distribution>/`. Filtered runs use the `focused/` subdirectory. Set `KEYRINGD_E2E_TRACE=keyringd_system_tests=trace` to write a detailed driver trace to `runtime/e2e-trace.log`.

`Runner::execute` assembles `build` and `execution`, and `Runner::execute_testbed` after promotion operates `environment`. Use [TestKit Runner API](../keyringd-testkit/README.md) to match the Cargo Artifact and Case/Result. Authenticator Build/Certificate/Server Lifecycle will be delegated to [Test-USBIP](../keyringd-test-usbip/README.md).

Use Case Evidence, Container Journal, Authenticator log, and the Bluetooth status of Host to check the failure locations. Successfulness alone does not determine physical TPM or real phone compatibility.
