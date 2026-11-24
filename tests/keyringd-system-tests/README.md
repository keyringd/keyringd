# system test

Combine the 3OS common case and the OS specific case into [Tests/Scenarios](tests/scenarios). A common case is defined only once, and the target OS is specified as `#[cfg(target_os = "...")]`. Place the case for verifying OS-specific authentication, configuration, and UI operations in the same tree and compile only the required OS.

The contract and OS implementation of the app operation are placed in [Application/Features](src/application/features). The assert of the test operation order, expected value, and result is determined by the scenario, and the operational type by function provides individual operations and observations from the actual UI/Fixture. A Page Object belongs to a Feature that uses that screen on a per-screen basis.

Registration, list, details, SSH connection test, About, and authentication screen Driver implement a common operation API directly, search and operate the elements of the actual screen. There is no page layer that wraps and delegates an operation type, or a state type that records the registration stage/screen opening/closing. The app owns an AT-SPI connection and passes `UiDriver` to each Driver to find the target UI. Each driver directly operates the input screen and the result screen of the connection test. The PIN transmission is returned after waiting for the answered input field to be no longer manipulated by the current UI.

Pass the same AT-SPI connection to the PIN/QR/Security Key/Device Key release authentication screen and do not reconnect after each operation. When the app is stopped, the search range for the terminated process is removed and the connection to the desktop is maintained. As a result, you can also search for authentication screens that the new process displays when SSH socket is activated.

The search and display observations of AT-SPI are read only once, and only the waiting API will retry the observation. The wait loop is summarized in a non-disclosed method of `UiDriver`, and restricts search, status check, and parent dialog acquisition with the same deadline. Do not retry any operations such as clicking. A separate upper limit of 2 seconds will be set to obtain a diagnosis after a standby failure.

The Window/Page of Feature will be in charge of Window/Page, which is specific to screen-specific element names and status, and `UiDriver` of each OS will be in charge of searching, inputting, waiting, and shooting. The Linux `AtspiSession` is responsible for AT-SPI connection, search range, tree cache and single-shot queries, and wait and diagnostic assembly is placed in `UiDriver`. The common operation of Windows UIA and Win32 and the macOS Fixture communication are summarized in each `UiDriver`, and the management of connection destinations and native resources is maintained for each OS. The common operation contract is TRAIT in Feature, and no additional TRAITs are provided to unify the drivers.

Windows/macOS `TestApp` manages process, startup settings, stopped mock credentials and restarts. The `SystemTestApplication` does not record the same startup status separately, delegate it to `TestApp`, and provides the Feature screen operation. The presence of the connection test screen is observed from the UI and does not record "open profiles" throughout the Application. The Windows Results dialog identifies the input screen from the native Owner.

The Windows wait API takes observations as `Result<Option<T>>` and retries only `Ok(None)` until the deadline. It returns `Err` immediately as an observation failure. Element lookups use one-shot APIs to avoid nested waits. Finding more than one match is an error.

## Responsibilities and Placement

```text
src/
├── application/
│   ├── features/
│   │   ├── about/
│   │   ├── navigation/
│   │   ├── profiles/
│   │   ├── profile_setup/
│   │   ├── qr_prompt_window/
│   │   ├── security_key_window/
│   │   ├── pin_prompt_window/
│   │   ├── ssh_connection_test/
│ │ ─ ─ PLATFORM/ # UIA, AT-SPI, Input/Shooting
├── ENVIRONMENT/ # Own/Start/Exit/Isolate the execution environment
├── Authenticator/ # USB, HYBRID, automation fixture
└── SSH/# SSH server client connection

tests/
├── main.rs
├── Scenario_Steps/
└── Scenarios/ # Test Case
    ├── application_menu/
    ├── service_management/
    ├── profile_management/
    ├── profile_setup/
    └── ssh_authentication/
```

Put a case definition on `tests/scenarios`. Preparation for use from multiple cases, operation procedure, observation, and verification of results
Place it on `tests/scenario_steps/{profile_setup,profile_management,ssh_authentication,ssh_connection_test}/`.
This is the layer that shares the OS-independent test procedures, and separates them from the layer that implements the OS-specific UI/process operations.
If you want to cut out from the case, specify the target OS and place it in the same location with the same responsibility.
The function group is grouped into a file representing the target/responsibility, and the files are public modules. The same description is not repeated with the module name and the function name, and the operation is indicated by the function name. Do not republish the function from the parent. Make the calls to the definition file like `scenario_steps::profile_management::device_key_replacement::approve(...)`.
`form` for `profile_setup` - `completion` - `passkey_not_found` is in charge of moving and canceling the entire wizard for `navigation`. `transport_choices` verifies the connection method and the Bluetooth state, and `security_key_prompt` verifies the display of waiting for key approval. `security_key` is responsible for the creation/recovery procedures for key requests, approval, PIN input, and `hybrid_profiles` and `security_key_profiles` shared by multiple cases. The initialization and connection of the authenticator will be indicated on the case side, and the preparation of the existing Profile will be specified in `profile_fixture`.
The SSH is `session.finish()` on the case side, and the waiting for the termination of the authentication screen is performed separately with `ssh_authentication::prompts::wait_closed`. The connection result screen verification is divided into `ssh_connection_test::result`, and the validation of the SSH request recorded by Fixture is divided into `ssh_connection_test::requests`.


Place a common trait/argument/observation type for each feature, and place the PAGE object on the inner `linux/`, `macos/`, and `windows/`. The targets of operations to be published are compatible with the actual Window, Wizard, Page, View. The number of people who simply delegate the same operation does not increase. The `application/platform/` has only low UI operating boards that do not know the specific Feature.

The owner of the execution environment is `LinuxSystemTestEnvironment`, `MacosSystemTestEnvironment`, and `WindowsSystemTestEnvironment` per OS. `NativeSystemTestEnvironment` selects the OS to run. `new()` has a quarantine environment and the app has not started yet. `application()` is in charge of app lifecycle and function, `authenticators()` is a virtual authenticator, and `ssh_servers()` is the SSH server. The environment itself does not implement a function-specific operation TRAIT.

Normal menu operation can be used with `ApplicationConfiguration::default()` for 3OS. Windows Fixture always launches the tray menu and isolates the storage of SSH settings to a registry key dedicated to the case. The quarantine key is inherited even after restarting and socket startup, and does not write to the actual user's `HKCU\Environment`. The case should not be enabled or specified as registry quarantine.

Authenticators are created with the `create_security_key(configuration)` and `create_hybrid_phone(configuration)` as many as required, and explicitly use the own handles returned in the case. It does not have an implicit state in the environment of "selected key" or "the other key". The common `AuthenticatorControl` is in charge of setting/required observation/approval, and operates from `key.authenticator()` and `phone.authenticator()` with the synchronization API. `UsbSecurityKey` is responsible for USB connection and disconnection, and `HybridPhone` is responsible for reading QR. The key retains the credentials even in `disconnect()` and `connect()`. `reset(profile)` erases the credentials and returns it to auto-approval. Do not mix authenticator operation into the UI operation type.

The USB connection/disconnection confirms the status after both the OS Attachment and the authenticator server are completed. Midway failures are kept as required, and the cleanup is completed before reconnection. There is no route to avoid connection failure by recreating the credentials.

The `SshClient` of the SSH client is common for 3OS and has a 30-second time limit from the collection/start of child PROCESS/standard/standard errors. Session assembles authentication results including server shutdown and log/public key information. Preparing and launching commands for SSHD is the responsibility of each OS.

The start of SSH returns an execution session that does not continue to borrow the environment, and the operation of the authentication screen and the waiting for termination are decided by the common case and the procedure side. The connection test also clearly shows the preparation, connection start, authentication, and the acquisition of the result, and there is no bulk-executable API that switches Transport with `bool`.

USB SSH authentication validates USB options and actual authentication, and does not assume QR for Bluetooth. Read, URI, and authentication of the QR are verified in a Hybrid-only case.

The list is `profiles_window()`, the details are returned by `open_details(name)`, and the deletion confirmation is operated as the window returned by `open_removal(name)`. The procedure of opening, operating, and closing a separate window is specified on the case or common step. The values read from the screen are returned in a common observation type, and the expected wording and state are compared on the common side.

Observe the form/completion screen for creating profiles, copy content/notifications of about public keys, and verify the expected values with `scenario_steps`. Expected values for dependent licenses by OS are also on the common side. The device key replacement is to confirm, start, select the authentication method, acquire the result, and finish the individual operation, and the approval of the authentication device and the verification of the results will be performed on the common side.

For the SSH connection test server, `environment.ssh_servers().start(trust)` returns the owner handle and operates with `client_configuration()`, `log()`, and `stop()`. Client configuration and save failure injections are explicitly passed to `application()`. UI function-specific operational types do not borrow server resources.

Create resources, launch apps, and open the screen on the scenario side:

```rust
let mut environment = NativeSystemTestEnvironment::new()?;
let key = environment.authenticators().create_security_key(Default::default())?;
environment.application().start(ApplicationConfiguration::default())?;
let mut window = environment.application().profile_setup_window();
window.open()?;
let mut wizard = window.wizard()?;
wizard.profile_form_page()?.enter_profile_name("example")?;
key.authenticator().set_approval_mode(ApprovalMode::Manual)?;
environment.application().capture_screenshot("Profile setup")?;
```

`capture_screenshot(label)` collectively performs trail registration for the shooting and current scenario procedures. In the operation of returning multiple observation results and images together, the image file is acquired with `screenshot()` and registered with the case side.

Profile Setup exposes `profile_form_page()`, `passkey_selection_page()`, `passkey_not_found_page()`, `interaction_page()`, and `completion_page()` through `profile_setup_window().wizard()?`. Use `profile_form_page()` for form input, `interaction_page()?.qr_view()` for switching to a USB key or scanning the QR code, and `completion_page()` for actions after completion. Creation, discovery, verification, and authorization are stages within the Interaction Page.

Wizard's `observe()` acquires the actual heading, wording, operational status, and the visible state of QR, including during page transitions. Summarize the waiting and expected values in `scenario_steps/profile_setup/progress.rs`. `form::submit_creation` • `form::submit_recovery` shares only the form input and submission, and the caller clearly indicates the operation and completion confirmation of the authenticator. Erase/restart profiles are divided into `application().reset_profiles()`, and subsequent operations are divided into Window's `open()`.

The OS-specific mechanism is placed in the role you are responsible for. Since the general-purpose SSH Fixture uses TestKit directly, it does not create a Linux-only `ssh/`. There will be no republish of the type for convenience to the route, and import directly from the modules to which it belongs. Common advance preparations for scenarios are placed near target functions such as `profile_setup/profile_fixture.rs`. There are no `drivers`s that are just a comprehensive `support` or Import.

## Run and development checks

Runners of each OS will prepare an environment such as an application and authenticator, and build and run one `scenarios` executable file for this crate. Filters and result matching also apply to this executable.

| Environment | All Cases and HTML Reports | Preparation and execution management |
| --- | --- | --- |
| Linux | `cargo linux-system-test-with-report` | [Linux runner](../keyringd-system-tests-linux-runner/README.md) |
| macOS desktop | `cargo macos-system-test-with-report` | [macOS runner](../keyringd-system-tests-macos-runner/README.md) |
| Windows desktop | `cargo windows-system-test-with-report` | [Windows runner](../keyringd-system-tests-windows-runner/README.md) |

`scenarios` is for explicit execution only and does not start with the normal `cargo test`. The above command is used for E2E because the runner's environment preparation, deadlines, and cleanup cannot be used when starting directly. Builds and tests usually specify the consolidated crate:

```sh
cargo build -p keyringd-system-tests --lib
cargo check -p keyringd-system-tests --test scenarios
cargo test -p keyringd-system-tests --lib
```

Specify `test-support` to explicitly verify macOS drivers and runner assistance on Linux. This is not an alternative to running the appkit GUI or the macOS real-world verification.

```sh
cargo check -p keyringd-system-tests --features test-support --all-targets
cargo test -p keyringd-system-tests --features test-support --lib
cargo check -p keyringd-system-tests-macos-runner --features test-support --all-targets
cargo test -p keyringd-system-tests-macos-runner --features test-support
```

The independent [WSL system tests](../keyringd-wsl-system-tests/README.md) verify the WSL packages.

## Common case verification contract

| File | common contract |
| --- | --- |
| `application_menu/about.rs` | Display and redisplay name/version, license-dependent libraries, background menus common to Mac/Windows, end, Mac standard menu |
| `service_management/configuration.rs` | Setting, canceling, canceling and canceling the usage environment of SSH Agent and confirming the use on the target OS |
| `service_management/socket_activation.rs` | Linux systemd shutdown/connection/reconnect to the same process and SSH authentication with a GUI-created Profile |
| `profile_management/identity_visibility.rs` | The same key/Comment reversal as the loss of all Identity when disabled |
| `profile_management/lifecycle.rs` | Rename, active status, delete, sort order and keep after reboot |
| `profile_management/device_key_replacement.rs` | Maintenance of CA/Passkey, Updated Device Key, Rejection of Old Direct Key and Real SSH |
| `profile_management/details_and_trust_entries.rs` | Display of Details and Content/Notifications of Individual/Bulk Copy |
| `profile_setup/form/validation.rs` | Blank name/Too long name rejection, retained input, no FIDO requests and saves |
| `profile_setup/form/cancellation.rs` | Cancellation before starting, return to list, no FIDO requests and no saves |
| `profile_setup/form/persistence.rs` | Removal of blank spaces before and after names and only one save |
| `profile_setup/form/storage_retry.rs` | Unsaved/non-disclosed when saving failed, retry with the same wizard |
| `profile_setup/create_new_passkey/security_key/success.rs` | Newly created with manual approval, only one save, real authenticator and real SSH |
| `profile_setup/create_new_passkey/security_key/configured_pin.rs` | 3OS common PIN Created with USB key, actual PIN input, RP, save, actual SSH |
| `profile_setup/recover_existing_passkey/security_key/configured_pin.rs` | 3OS common PIN recovery to new PC, same CA Passkey and new Device Key Real SSH |
| `ssh_authentication/connection_test/identity_selection.rs` | Identity exclusive selection for 3OS common initial selection, switching, and reselect |
| `profile_management/profile_order_menu.rs` | 3OS common up and down movement, boundary invalid state, order of save, and retained after restarting |
| `profile_setup/create_new_passkey/security_key/multiple_keys.rs` | Create with the second key, recover without other keys, maintain CA/Passkey and SSH |
| `profile_setup/create_new_passkey/security_key/cancellation.rs` | Cancellation of actual requests, unsaved/unpublished, recreated with the same key |
| `profile_setup/recover_existing_passkey/security_key/` | Search cancellation and rerun, multiple candidate selection and CA/Passkey collation, resumption after cancellation, retry from absence, real SSH with the same CA and the new Device Key |
| `profile_setup/create_new_passkey/hybrid/` | Created, QR update, recreate after cancellation in real Hybrid/PXP |
| `profile_setup/recover_existing_passkey/hybrid/` | Cancel search, noCredentials response of real phone, switch to new creation from absence, select multiple candidates and CA/Passkey collation, restart after cancellation (real HYBRID communication) |
| `profile_setup/recover_existing_passkey/hybrid/success.rs` | Recovering from the same phone, maintaining CA, new Device Key, real SSH to the original CA |
| `ssh_authentication/ssh_command/multiple_profiles.rs` | Success in each of the two CAs, refusal to present only the certificate of another CA |
| `ssh_authentication/ssh_command/` | Real SSH with Device Certificate/Device Key, Accepted Identity, Standard Output, Server Logs and FIDO Unused |
| `profile_setup/create_new_passkey/hybrid/availability.rs`・`recover_existing_passkey/hybrid/availability.rs` | Bluetooth Disabled/Adapter Absence, USB Selection |
| `ssh_authentication/ssh_command/passkey.rs`・`hybrid_passkey.rs` | Real SSH, Continuous Authentication, Authorization Request and Accepted Identity, Standard Output, Server Log on USB/Hybrid |
| `ssh_authentication/connection_test/` | Success of 4 types of Identity, mismatch of trust key/CA/user name, display result and copy registration information |
| `profile_setup/create_new_passkey/security_key/mixed_verification.rs` | Mixed UV/PIN on UNIX, token permissions and re-entry of the selection key, canceling the request for non-selected keys |
| `ssh_authentication/ssh_command/security_key_pin.rs` | Real SSH Certification with PIN during or after closing profile list on all OSs (PIN authentication cache by OS allowed) |

Drugs remain non-supported in Windows elevation. In macOS products
Pass the Pasteboard to the actual Drop Handler. Even if the operation routes are different for each OS, the display order and
The common case defines the expected value for the order of saving and the order after restarting.

The `security_key/user_verification.rs` created and restored shares built-in UV priority and PIN settings/inputs on Linux/Mac.
The type of token, RP scope, `mc`, and `ga` privileges that the product FidoClient directly selects.
Validate UV display. On Windows, the WebAuthn API uses a Token acquisition and a PIN cache.
For management, the common config_pin case is used to observe the presence or absence of a PIN input, and does not fix the number of re-entry for Windows. 3OS common success
Do not force UNIX CTAP request order or TOKEN scope.

## Linux

Validate the GTK screen and SSH authentication of installed Linux products. [Runner](../keyringd-system-tests-linux-runner/README.md) is in charge of building and managing the environment.

### Execution and Environmental Contracts

```sh
cargo linux-system-test-with-report
cargo linux-system-test-with-report --distribution fedora
cargo linux-system-test-with-report --filter profile_management
```

`scenarios` Target is for explicit execution only and does not work with the normal `cargo test`. Runner builds and executes Target in the environment it has prepared. Tests itself does not rely on the NSPawn API.

| Input | Necessary environment |
| --- | --- |
| Installed package | Product Binary and Systemd User Units |
| `DISPLAY` / session bus | X11 and AT-SPI available |
| `TPM2TOOLS_TCTI` | Connect to Test TPM |
| HID | Accessing FIDO2 Key |
| `KEYRINGD_E2E_USB_CONTROL` | Dynamic USB/IP connection service owned by the runner. Responsible for connecting, disconnecting, and collecting keys made from a case |
| `XDG_RUNTIME_DIR` | Systemd User Session and Agent Socket |
| `KEYRINGD_E2E_ROOT` | Isolated Writable State Directory |
| `KEYRINGD_E2E_ARTIFACTS` | Trailing destinations as needed |
| `KEYRINGD_E2E_AUTHENTICATOR_CONTROL` | Case Control API with Virtual Authenticator |

### Guaranteed route

Find Linux-specific scenarios in [tests/scenarios](tests/scenarios), page objects in [application/features](src/application/features), AT-SPI and X11 operations in [platform/linux](src/application/platform/linux), environment and app launch code in [environment/linux](src/environment/linux), and virtual keys and BLE advertisements in [authenticator/linux](src/authenticator/linux). Together with the shared cases, these cover profile creation, recovery and management, OpenSSH configuration, connection tests, and real SSH with a Device Certificate, Device Key, or Passkey.

Testbed typically uses the software TPM and the virtual USB/IP key through installed Packages and Systemd Sockets. Hybrid is separated by PXP Relay and BtVirt/BlueZ. `hci_vhci` Absence should be SKIP with reason for BLE CASE. Relay success does not mean success of the BlueZ route. The Scanner command for another machine itself is not validated.

The conditions for which the keys are not available are to temporarily remove the access rights of the HiDraw nodes owned by Runner. Confirm that OPEN is denied by the same E2E user as the app, and returns it to its original permissions when exited. This is not a physical unplugging validation because it maintains a USB/IP connection.

QR is read from PNG on the screen with Pure Rust Decoder on TestKit. Virtual Phone operations use Test-USBIP and TestKit's `PxpAdvertisementRelay` for sending ads to Agents.

### UI operation contract

Page objects restrict exploration to the screen or the descendant of the Dialog, and use the Accessible Name, Role, and Display states published by GTK. The input field is Text/PasswordText, and the operational buttons are displayed and active, and the action is confirmed.

- `wait_for_*` returns the page object of the transition destination.
- `assert_*` verifies the expected value/state within the deadline.
- The operation also checks the screen transitions and completions.
- Elements absent only if it can be searched normally. Do not regard communication errors as absent.
- The screen shots are trails, and the assertions of the displayed content are done separately.

The AT-SPI Driver uses the target bus name Cache and Event. After the structure change, check the elements that can be reached from the current window. Update the connection destination and tree by restarting the app.

### Result

HTML is `target/test-reports/linux-system/<distribution>/index.html`, `focused/index.html` for filter execution. Leave JSON, screen, Journal, SSH/TPM/Authenticator logs to the same Suite Directory. For the number of cases and the reason for SKIP, refer to the report of the target execution.

## macOS

Operate the product AppKit screen with the NSAccessibility Driver in the Fixture. Use the product UI, Command Handler, Core Workflow, Dedicated Agent, and Product ProfileRepository, and configure the key and FIDO Backend for each case.

[Runner](../keyringd-system-tests-macos-runner/README.md) is in charge of environment, server, expiration, and process cleanup.

### Execution

Prepare macOS Rust Toolchain, Desktop, JDK, SSH/SSHD/SSH-KEYGEN.

```sh
cargo macos-system-test-with-report
cargo macos-system-test-with-report --filter profile_management
cargo macos-system-test-with-report --case-timeout-seconds 240
```

Since the fixture itself operates the NsAccessibility hierarchy, no external accessibility is required. Screen png is also drawn from the backing store of the app, no screen recording permission is required. It is different from the trail that the actual screen was taken from Windowserver.

macOS and Windows Hybrid/PXP Case explicitly enable advertising relaying on the startup `accept_remote_hybrid_advertisements`. Pass the `--accept-remote-hybrid-advertisements` argument to the macOS Fixture. Environment variables that specify the connection destination and CA of the test HYBRID tunnel do not enable advertising relaying.

Normal `cargo test` does not start the explicit SCENARIO. Since you can't use Runner's entire case expiration date when you start Scenario Executable directly, you can't use the public command.

### Verification route

| Case | FIDO/SSH route | Limits of Warranty |
| --- | --- | --- |
| USB | Products FidoClient + UserSpace USB/IP + Virtual Authenticator | Physical HID/OS equipment Permission does not pass |
| HYBRID communication | Screen QR + Virtual Phone + Test TLS Tunnel + Product Noise / CTAP | Real Phone and Real BLE Scan are not guaranteed |
| UI notification/abnormal system | Specify a wait, fail, cancel, and bluetooth state in TestPassKeyService | Not used as evidence of real communication |
| SSH | Socket Agent + Real SSH/SSHD | DEVICE Key is a software implementation |
| launchd | Actual user GUI DOMAIN, SecureSocketWithKey, Setting/Restore Default Agent, Real SSH Authentication on the GUI Created Profile | The basic launch contract is a temporary LaunchAgent, and the SSH certification uses the product UI Fixture and Software Device Key. Real-time Secure Enclave is not guaranteed |

macOS specific case is [Tests/Scenarios](tests/scenarios). Together with the common case, create, recover, select, manage, and reboot the public key copy, Device Key authorization, connection test and actual SSH. The SSH results are not injected into the screen and the results of the dedicated OpenSSH Process are displayed.

Secure Enclave, KeyChain, LaunchD settings after OS reboot, and physical Authenticator interoperability are outside the scope of this Fixture verification and are checked separately on the actual machine.

### Isolation and cleanup

Profile JSON, Agent Socket, Home and OpenSSH settings are for case only. The product does not normally write to Application Support, Keychain, or user `~/.ssh`. Use the dedicated Pasteboard for the Copy operation of Fixture. Dedicated HOME INCLUDE uses Absolute PATH, and SSH diagnostics without setting do not reference real HOME in `-F /dev/null`.

Runner monitors the case in a default of 180 seconds and terminates the post-defense RPC. Finish the Process Group and collect the State and Residual Authenticator. Unrecoverable Cleanup Failure to proceed to the next case.

### UI Search and Assertion

The Driver entity is placed on the `test-support` Feature of the product. Communicate with Fixture in a shared RPC type, and search for product ownership elements with a stable identifier and target window. Only the inside of the standard about inside the indications owned by the AppKit is used as a clue.

The [macOS implementation of each feature](src/application/features) defines its own page objects. Setup has separate steps for the form, transport, PIN, credential selection, and completion. The `assert_*` methods check headings, displayed values, and enabled states. Operations also check selections and whether a window closed.

RPC Failure, multiple candidates, different types, and search limits are errors. Do not convert communication failures to elements absent. Waiting has a deadline, leaving the last observation and the AX tree in the diagnosis.

### Trails and normal tests

HTML is `target/test-reports/macos-system/index.html` and filter is `focused/index.html`. You can view case screens, logs, profile JSON, AX diagnostics, SSH/Authenticator trail and execution environment. Selected Case Skip is also handled by Runner as FAIL.

```sh
cargo test -p keyringd-system-tests --lib
```

This normal test confirms the platform-independent driver contracts and support processes. The success of the GUI refers to the System Test results on macOS.

The `service_management` MAC case temporarily disables the standard Agent of the real login session. The fixture is restored when the normal end/PANIC is performed, and when the forced termination or expiration date is overdue, the RUNNER is recorded and restored. If the restore fails, it will fail the test and keep the state files required for recovery.

To create and restore a PIN set key, use `security_key::approve_step_with_pin`, which operates one PIN UI of the OS, which is authenticated at a time.
The operation result is returned as the presence or absence of PIN input, and the common case verifies the transition, save, and SSH authentication.
Requires PIN input when creating the original PC. Windows can reuse the verification results at the time of creation in recovery,
Recording whether or not the inputs are made at the time of recovery is not requested for re-entry each time. Linux/Mac CTAP privileges and token types
Keep the specification case to check separately and do not request the same CTAP request order from Windows.

## Service management

Linux socket activation scenarios use `SocketActivationDriver` to prepare a systemd service, obtain its socket path, and check process reuse. `prepare_application_socket_activation()` retains the GUI-created Profile while replacing the running application with a socket-activated service.

macOS starts the app at login and the app owns its socket. Its UI fixture opens a case-private socket and uses a separate case-private original agent socket for Configure and Unconfigure. Tests verify that the original path becomes a link to Keyringd, survives an app restart, and is restored to the original socket without changing the client environment. The fixture does not alter the user's launchd jobs or SSH agent socket. Socket regression tests live alongside each OS listener in `system/socket`; macOS socket selection tests live in `keyringd-macos/src/system/open_ssh/ssh_agent_selection.rs`.

## Windows

Operate the product WinUI screen from a separate Process UI Automation. [Runner](../keyringd-system-tests-windows-runner/README.md) is in charge of environment, Build, sharing Server, and execution management.

### execution environment

Rust MSVC, Visual Studio C++ Build Tools, Windows SDK, Interactive Desktop, JDK, USBIP-Win2, Windows OpenSSH Client/Server, Git for Windows required. The Git included SSH is selected from `%ProgramFiles%\Git\usr\bin`. Deploy OpenSSH with Admin PowerShell:

```powershell
Add-WindowsCapability -Online -Name OpenSSH.Client~~~~0.0.1.0
Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0
```

Use ssh, sshd, and ssh-keygen from `%SystemRoot%\System32\OpenSSH`. Since the tests start SSHD directly with the dedicated settings and the free port of Loopback, no sshd service is required.

For the CI installation of USBIP-WIN2, refer to [Windows System Test Workflow](../../.github/workflows/system-tests-windows.yml) installation step]. Runner uses the existing Client/Controller and does not introduce the Driver in the test run.

```powershell
cargo windows-system-test-with-report
cargo windows-system-test-with-report --filter validation
```

The normal `cargo test` does not start the explicit SCENARIO. GUI Case runs sequentially. CNG/WebAuthn PIN operation requires a UAC promotion to a dedicated Automation Process.

### Routes to be verified

| Case | Route |
| --- | --- |
| USB creation/recovery/cancel/multiple key/pin | Attach virtual keys on USBIP-Win2 and use the product Windows WebAuthn API |
| HYBRID communication | Screen QR, Virtual Phone, Test TLS Tunnel, Product FIDO Client |
| UI notification/abnormal system | Create a Bluetooth state, pending, failure, noCredentials in TestPassKeyService |
| SSH | The same fixed as the product named Pipe Agent, real ssh.exe, temporary sshd for the same user |
| Git for Windows SSH | Products MSYS-compatible Listener, Git included ssh.exe, temporary sshd for the same user. Also check for use with Windows SSH and restart Agents |
| Local authentication | Dedicated Software KSP and CNG Strong Key Protection UI |

The product UI Fixture is on the `test-support` Feature of `keyringd-windows`. Use the Product ProfilesWindow, Setup, Command Handler, Application/Workflow. The Device Key is a Software implementation and is not guaranteed by the physical TPM. Even after rebooting, use the case dedicated Store and Test Handle.

Do not pin the internal CTAP order because Windows determines PIN/UV and TOKEN reuse. The search and operation of the OS screen will be delegated to the promoted Automation Process. WebAuthn is a keyringd relying party display on the screen, and CNG is uniquely matched with the description of the target key. In CI, the owner chain of the OS screen does not connect to the HWND of the fixture, so the owner chain is not used as a search condition. The HTTP approval of the virtual key advances the Authenticator side of the CONSENT.

### Isolation and fixture

Each case owns its profile repository, OpenSSH configuration, host key, and public key registration entry. Cases run serially and use the product pipe `\\.\pipe\keyringd-agent-<SID>`. The agent stops after each case. The suite aborts if the regular app is running or the pipe is unavailable. Getting Started tests use a dedicated HKCU key with a UUID to preserve the user’s Environment key. The UI shows the connection result from the real SSH process.

The independent [WSL system tests](../keyringd-wsl-system-tests/README.md) verify WSL package installation and SSH authentication over WSL. The suites share the runner infrastructure.

### Scenario and Driver

The Windows specific case is [Tests/Scenarios](tests/scenarios). Create/Recover/Select, manage and reboot with common case, Copy, Device Key Replacement, OpenSSH configuration, connect test, Device Certificate/Device Key/Passkey Validate the real SSH in.

The [Windows implementation of each feature](src/application/features) defines a page object per screen. The driver uses a dedicated COM MTA thread to find elements within the process and window, run Value, Invoke, and Toggle operations, wait, and capture PNGs. It uses Windows.Graphics.Capture and waits for rendering before saving. Keep actions and `assert_*` methods separate, with explicit checks for displayed values, enabled states, and completion.

The Windows specific case is also placed in the target feature scenario. Define the order and expected values for creating/recovery of PINs in the 3OS common case. Windows Security operation/waiting is set to Page Object, and Agent Identity is acquired in [SSH](src/ssh/windows). Specify the identity to get with the product's `ProfileSshIdentityKind`.

### Results and warranty coverage

HTML is `target/test-reports/windows-system/index.html` and filter is `focused/index.html`. Leave the screen, public key registration line, SSH authentication log, and the required Automation diagnosis on the trail. FIDO's real communication and TestPassKeyService Cases are described in their respective descriptions.

Real TPM, Real Phone, Ble Scanner, and Distribution MSI deployments are outside the scope of this Fixture. Check `security-automation.log` and `process-security.log` when searching for OS Dialog fails. Differentiate between the platform-independent support processing that is normally tested and the UI case that passes through Windows Desktop.

Device Key Unavailable cases are specified in `ApplicationConfiguration::device_key_unavailable`. On Linux, the TPM device, which is not only available in the systemd service of the app, is set as a TCTI connection destination and restored when it is stopped. The Test Device Key backend returns unavailable on Windows/macOS. Do not change the host's TPM/Secure Enclave settings. Make sure you have a hardware banner when you open the Profiles list, disable the Add button, and you can add it when you open it again with the available settings.
