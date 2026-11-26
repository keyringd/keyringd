<h1><img src="docs/images/logo.svg" alt="Keyringd" width="600"></h1>

[![CI](https://github.com/keyringd/keyringd/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/keyringd/keyringd/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/keyringd/keyringd?sort=semver)](https://github.com/keyringd/keyringd/releases/latest)
[![License](https://img.shields.io/github/license/keyringd/keyringd)](LICENSE)

Keyringd is an SSH agent that keeps your private keys in your computer’s hardware.

Hardware-protected keys can’t be extracted by an attacker, but they’re bound to a single device. Normally, replacing your computer means creating a new key and registering it with every server. With Keyringd, you don’t have to start over.

Instead of transferring the private key, Keyringd uses your passkey as an SSH certificate authority (CA) to certify a new Device Key for your next computer. Once your servers trust that CA, their setup stays the same across computers.

For everyday SSH, your Device Key does the work—without asking for your passkey each time.

[Documentation](https://keyringd.com/) · [Installation](https://keyringd.com/installation/) · [Set up Keyringd](https://keyringd.com/setup/) · [Releases](https://github.com/keyringd/keyringd/releases) · [Issues](https://github.com/keyringd/keyringd/issues)

## Features

- **Hardware-backed Device Keys.** Generate and sign with a private key held by the computer's TPM or Secure Enclave.
- **Passkey-based authorization.** Use a passkey to issue OpenSSH certificates for your computers. Everyday certificate authentication signs locally with the Device Key.
- **CA and direct-key authentication.** Use certificates where the destination accepts an SSH CA, or register a Device Key or passkey directly for services that don't accept SSH certificates.
- **USB and phone passkeys.** Use a FIDO2 security key over USB or a passkey on a phone through a QR-initiated Hybrid connection.
- **Native applications.** Manage Profiles and keys through GTK, WinUI, or AppKit. A WSL2 proxy connects Linux SSH clients to the Windows agent.
- **Optional local authentication.** Require authentication when the Device Key signs, using the platform's key protection mechanism.
- **SSH signing.** Use Device Keys with Git's SSH signing support; verifier trust is configured separately from SSH login.

A **Profile** associates a passkey with this computer's Device Key, its certificate, and settings. The passkey authorizes new Device Keys; it is not needed for everyday Device Key signatures.

## Platforms

| Platform | Device Key storage | UI toolkit |
| --- | --- | --- |
| Ubuntu / Fedora | TPM 2.0 | GTK / libadwaita |
| Windows 11 version 23H2 or later | TPM 2.0 | WinUI |
| macOS 13 or later (Apple silicon only) | Secure Enclave | AppKit |
| WSL2 | Windows agent's TPM-backed key | WinUI (Windows companion) |

See [Installation](https://keyringd.com/installation/) for full platform and authenticator requirements.

## Get started

1. Install the package for your platform and connect your SSH client to the agent.
2. Create a Profile using a new or existing passkey.
3. Register the CA public key or direct public keys required by your destination.
4. Connect with your usual SSH client.

The [installation guide](https://keyringd.com/installation/) installs Keyringd and connects the SSH client to its agent. [Set up Keyringd](https://keyringd.com/setup/) then walks through one working connection. [Public key registration](https://keyringd.com/guides/public-key-registration/) explains the destination-specific choices. [Usage](https://keyringd.com/usage/) covers Profile management, and [Git signing](https://keyringd.com/guides/git-signing/) covers signing and verification.

Hardware protection prevents private-key extraction; access to the agent can still permit signing. Read the [Security Model](https://keyringd.com/security-model/) for local authentication, forwarded agents, Passkey authority, and revocation.

## Build

The repository contains the Rust workspace, native applications, test infrastructure, packaging, and documentation site. Use Rust stable and the native toolchain for your target OS.

For example, on a configured Windows development host:

```powershell
cargo xtask build --locked
cargo run -p keyringd-windows
```

## License

Keyringd is licensed under the [Apache License 2.0](LICENSE). Dependency notices are generated for application builds.
