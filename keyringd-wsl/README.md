# keyringd-wsl

A Linux Process that relays the WSL's UNIX Socket and the Windows Agent. Deployment and SSH settings are [Installation](https://keyringd.com/installation/#_wsl2) of the site, here dealing with relay implementation, development, and verification.

The first Socket connection starts Proxy Service, and the included Windows Companion `bridge` will automatically start the Agent as needed.

The Windows runtime package preserves the exact spelling that the loader requires for `Microsoft.UI.Xaml.Resources.19H1.dll`, `Microsoft.UI.Xaml.resources.common.dll`, and `en-US`. The SDK uses different casing, which works on Windows but causes theme and string resources to fail to load on the WSL filesystem. `cargo dist --format runtime --target windows` fixes the casing when packaging the runtime.

```text
Unix socket connection
  → keyringd-wsl-proxy
  → Keyringd.exe Bridge Process per connection
  → Windows agent
```

The Windows side owns the Profile, Key, FIDO Authentication, and Confirmation screen. Proxy does not interpret the Agent Frame and transfers the byte sequences in two directions.

## Implementation ownership

| Mold | Duty |
| --- | --- |
| [CommandLine](src/app/command_line.rs) | Specify Windows executable file |
| [WslEntryPoint](src/app/wsl_entry_point.rs) | Initialize and start of arguments and logs |
| [WslProxyHandler](src/app/handler/wsl_proxy_handler.rs) | Connection acceptance, connection Task, end of IDLE |
| [UnixSocketListener](src/system/unix_socket_listener.rs) | Receipt of Systemd Descriptor, owning Socket when starting alone |
| [WindowsBridgeRelay](src/system/windows_bridge_relay.rs) | Starting, relaying, ending/collecting the Bridge Process |

Place OS resources in System/ and start/connection management in APP/. Make sure Bridge Process and Task do not remain even when the connection is terminated or COPY fails.

## Socket contract

[Package Socket Unit](../packaging/wsl/rootfs/usr/lib/systemd/user/keyringd.socket) creates a Socket and passes it to [Service](../packaging/wsl/rootfs/usr/lib/systemd/user/keyringd.service)." The proxy does not remove the systemd Socket Path. Process ends with an IDLE of 60 seconds after all connections are finished, and restarts on the next connection.

Create `$XDG_RUNTIME_DIR/keyringd-agent.sock` for single startup, or use `/tmp/keyringd-$USER.sock` if you don't (user if you haven't set it). Permission is 0600. Do not overwrite the Socket, normal file, Symlink, and replace only the old Sockets that cannot be connected. There is no idle termination in the single startup, and the Socket created by you is deleted at the time of DROP.

Native Linux Package and WSL Package use the same socket path. It does not conflict with the existing Socket Activation even when it starts alone during development.

## Starts on development

Specify the Windows Build executable file on Linux/WSL.

```sh
cargo run -p keyringd-wsl -- --windows-exe /mnt/c/path/to/keyringd.exe
```

By default, use the same directory keyringd.exe as the proxy. Package Service explicitly indicates /usr/libexec/keyringd/keyringd.exe. Package generation requires a proxy and a Windows Runtime, and the input is referenced to [XTask](../xtask/README.md).

## Verification

Runs on Linux/WSL.

```sh
cargo test -p keyringd-wsl
cargo test -p keyringd-wsl --features test-support
```

Normally, test is the executable file detection and the Endpoint specification, and the `test-support` also checks the Socket Life/Relay Process End/Idle end. The product's signature policy and key service verification are validated by the product.

The independent [WSL system tests](../tests/keyringd-wsl-system-tests/README.md) verify installation, public key enumeration, SSH authentication with all three identity types, rejection, and reconnection on WSL 2. They use the companion and proxy from the WSL package. Dedicated CI jobs test the Ubuntu deb and Fedora rpm.
