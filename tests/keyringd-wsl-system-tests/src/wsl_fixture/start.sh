set -euo pipefail
directory=$1
case "$directory" in /root/keyringd-e2e-*) ;; *) exit 1 ;; esac
test -f /etc/keyringd-e2e-disposable
proxy=/usr/bin/keyringd-wsl-proxy
test -x "$proxy"
test -x /usr/libexec/keyringd/keyringd.exe
export XDG_RUNTIME_DIR=/run/user/0
loginctl enable-linger root
systemctl start user@0.service
systemctl --user stop keyringd.service keyringd.socket
systemctl --user reset-failed keyringd.service
# Preserve WSL interop and isolated fixture settings in the socket-activated service.
systemctl --user import-environment WSL_INTEROP WSLENV
if test -n "${LLVM_PROFILE_FILE:-}"; then
    systemctl --user import-environment LLVM_PROFILE_FILE
fi
for name in KEYRINGD_UI_TEST_CONTROL_FILE KEYRINGD_UI_TEST_STATE_FILE KEYRINGD_UI_TEST_DEVICE_KEY_LEDGER KEYRINGD_UI_TEST_SSH_CONFIGURATION_REGISTRY_KEY KEYRINGD_UI_TEST_FIDO_BACKEND KEYRINGD_UI_TEST_DEVICE_KEY_BACKEND KEYRINGD_UI_TEST_HYBRID_TUNNEL_ADDRESS KEYRINGD_UI_TEST_HYBRID_TUNNEL_CA; do
    systemctl --user unset-environment "$name"
    if test -n "${!name:-}"; then systemctl --user import-environment "$name"; fi
done
mkdir -m 700 "$directory"
printf '%s\n' 'preparing SSH keys and configuration' > "$directory/setup-stage"
printf '%s\n' "$2" > "$directory/authorized_keys"
printf '%s\n' "$3" > "$directory/identity.pub"
ssh-keygen -q -t ed25519 -N '' -f "$directory/host_key"
mkdir -p /run/sshd
port=$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')
cat > "$directory/sshd_config" <<EOF
ListenAddress 127.0.0.1
Port $port
HostKey $directory/host_key
PidFile $directory/sshd.pid
AuthorizedKeysFile $directory/authorized_keys
StrictModes yes
PermitRootLogin prohibit-password
PubkeyAuthentication yes
PasswordAuthentication no
KbdInteractiveAuthentication no
AuthenticationMethods publickey
UsePAM yes
LogLevel VERBOSE
EOF
cat > "$directory/ssh_config" <<EOF
Host fixture
    HostName 127.0.0.1
    Port $port
    User root
    IdentityAgent /run/user/0/keyringd-agent.sock
    IdentityFile $directory/identity.pub
    IdentitiesOnly yes
    PreferredAuthentications publickey
    BatchMode yes
    StrictHostKeyChecking yes
    UserKnownHostsFile $directory/known_hosts
    GlobalKnownHostsFile /dev/null
    ConnectTimeout 10
EOF
printf '[127.0.0.1]:%s %s\n' "$port" "$(cut -d ' ' -f 1,2 "$directory/host_key.pub")" > "$directory/known_hosts"
printf '%s\n' 'starting sshd and package socket only' > "$directory/setup-stage"
/usr/sbin/sshd -D -e -f "$directory/sshd_config" </dev/null >"$directory/sshd.log" 2>&1 &
echo $! > "$directory/sshd-process.pid"
systemctl --user start keyringd.socket
test "$(systemctl --user show keyringd.service --property=MainPID --value)" = 0
systemctl --user is-active keyringd.socket > "$directory/socket-before-connect"
printf '%s\n' 'waiting for the proxy socket and sshd listener' > "$directory/setup-stage"
# The Windows owner enforces the setup timeout, including cold WSL startup.
while true; do
    kill -0 "$(cat "$directory/sshd-process.pid")"
    if test -S /run/user/0/keyringd-agent.sock && grep -q 'Server listening' "$directory/sshd.log"; then
        printf '%s\n' ready > "$directory/setup-stage"
        printf '%s\n' KEYRINGD_WSL_READY
        # Keep the wsl.exe interop session alive while the proxy starts Windows bridges.
        sleep infinity
        exit $?
    fi
    sleep .05
done
