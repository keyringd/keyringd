# Security Policy

## Supported Versions

Security fixes for Keyringd are provided for the latest release.

## Reporting a Vulnerability

If you suspect a security vulnerability, do not report it through a public GitHub issue, discussion, or pull request.

Use GitHub's private vulnerability reporting:

1. Open the [Security Advisories page][advisories].
2. Select **Report a vulnerability**.
3. Submit the report privately.

Where possible, include the following information:

- The affected Keyringd version or commit.
- The operating system and version.
- The installation method and relevant configuration.
- The authenticator or passkey provider involved.
- The expected behavior and the observed behavior.
- The potential security impact.
- The conditions or privileges required for the attack.
- Complete reproduction steps or a proof of concept.
- Whether the issue has already been disclosed publicly or shared with other parties.

Do not include secrets or personal information that are not necessary to reproduce the issue, or information obtained from systems you do not own or administer.

## What to Expect

We aim to provide an initial response within 14 days of receiving a report.

We will investigate the report and request additional information when needed. We will keep the reporter informed of significant progress.

If a vulnerability is confirmed, we will coordinate the fix and disclosure timeline with the reporter and publish a GitHub Security Advisory.

Please keep the report confidential until we agree that disclosure is appropriate. With the reporter's permission, we will credit their contribution in the Security Advisory.

## Security Model

Keyringd's security goals, assumptions, and known limitations are documented in the [Security Model][security-model].

Behavior consistent with a documented limitation may not, by itself, constitute a vulnerability. However, reports demonstrating impact beyond the documented model, unexpected behavior, or a way to bypass an intended protection are welcome.

If you are unsure whether an issue qualifies as a vulnerability, please report it privately.

[advisories]: https://github.com/keyringd/keyringd/security/advisories
[security-model]: https://keyringd.com/security-model/
