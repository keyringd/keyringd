Name: keyringd-wsl
Version: %{pkg_version}
Release: %{pkg_release}
Summary: Passkey-backed SSH agent for Windows and WSL
License: Apache-2.0
URL: https://github.com/keyringd/keyringd
BuildArch: x86_64
Conflicts: keyringd
Source0: keyringd-wsl-proxy
Source1: windows-runtime
Source2: rootfs
Source3: LICENSE
BuildRequires: systemd-rpm-macros
Requires(post): systemd

%description
Keyringd combines a TPM-backed SSH key with a FIDO authenticator and an
OpenSSH certificate. This package includes the Linux WSL proxy and its native
Windows companion executable.

%install
cp -a %{SOURCE2}/. %{buildroot}/
install -d %{buildroot}%{_bindir} %{buildroot}%{_libexecdir}/keyringd \
  %{buildroot}%{_licensedir}/keyringd
install -m 0755 %{SOURCE0} %{buildroot}%{_bindir}/keyringd-wsl-proxy
cp -a %{SOURCE1}/. %{buildroot}%{_libexecdir}/keyringd/
install -m 0644 %{SOURCE3} %{buildroot}%{_licensedir}/keyringd/LICENSE

%post
%systemd_user_post keyringd.socket
if [ "$1" -eq 1 ]; then
    # Start the enabled socket in user managers that are already running.
    systemctl list-units 'user@*.service' --state=active --plain --no-legend --no-pager 2>/dev/null |
        while read -r unit _; do
            uid=${unit#user@}
            uid=${uid%.service}
            if systemctl --user --machine="$uid@" daemon-reload &&
                systemctl --user --machine="$uid@" is-enabled --quiet keyringd.socket; then
                systemctl --user --machine="$uid@" start keyringd.socket || :
            fi
        done
fi

%preun
%systemd_user_preun keyringd.socket

%postun
%systemd_user_postun keyringd.socket

%files
%{_bindir}/keyringd-wsl-proxy
%{_libexecdir}/keyringd
%{_userunitdir}/keyringd.socket
%{_userunitdir}/keyringd.service
%{_userpresetdir}/90-keyringd.preset
%license %{_licensedir}/keyringd/LICENSE
