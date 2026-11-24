Name: keyringd
Version: %{pkg_version}
Release: %{pkg_release}
Summary: Native Linux desktop passkey-backed SSH agent
License: Apache-2.0
URL: https://github.com/keyringd/keyringd
BuildArch: x86_64
Source0: keyringd
Source1: rootfs
Source2: LICENSE
Conflicts: keyringd-wsl
Requires: /usr/bin/pkexec
Requires: shadow-utils

%description
Native Linux desktop SSH agent using TPM device keys and FIDO passkey credentials.

%install
cp -a %{SOURCE1}/. %{buildroot}/
chmod 0755 %{buildroot}%{_libexecdir}/keyringd/grant-tpm-access
install -d %{buildroot}%{_bindir} %{buildroot}%{_licensedir}/keyringd
install -m 0755 %{SOURCE0} %{buildroot}%{_bindir}/keyringd
install -m 0644 %{SOURCE2} %{buildroot}%{_licensedir}/keyringd/LICENSE

%post
%systemd_user_post keyringd-agent.socket

%preun
%systemd_user_preun keyringd-agent.socket

%postun
%systemd_user_postun keyringd-agent.socket

%files
%{_bindir}/keyringd
%dir %{_libexecdir}/keyringd
%{_libexecdir}/keyringd/grant-tpm-access
%{_datadir}/polkit-1/actions/com.keyringd.grant-tpm-access.policy
%{_userunitdir}/keyringd-agent.socket
%{_userunitdir}/keyringd-desktop.service
%{_datadir}/applications/com.keyringd.Keyringd.desktop
%{_datadir}/dbus-1/services/com.keyringd.Keyringd.service
%{_datadir}/icons/hicolor/scalable/apps/com.keyringd.Keyringd.svg
%{_userpresetdir}/90-keyringd-desktop.preset
%license %{_licensedir}/keyringd/LICENSE
