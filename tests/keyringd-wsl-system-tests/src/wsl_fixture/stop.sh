set -euo pipefail
directory=$1
case "$directory" in /root/keyringd-e2e-*) ;; *) exit 1 ;; esac
test -f /etc/keyringd-e2e-disposable
export XDG_RUNTIME_DIR=/run/user/0
systemctl --user stop keyringd.service keyringd.socket
systemctl --user unset-environment KEYRINGD_UI_TEST_CONTROL_FILE KEYRINGD_UI_TEST_STATE_FILE KEYRINGD_UI_TEST_DEVICE_KEY_LEDGER KEYRINGD_UI_TEST_SSH_CONFIGURATION_REGISTRY_KEY KEYRINGD_UI_TEST_FIDO_BACKEND KEYRINGD_UI_TEST_DEVICE_KEY_BACKEND KEYRINGD_UI_TEST_HYBRID_TUNNEL_ADDRESS KEYRINGD_UI_TEST_HYBRID_TUNNEL_CA
for file in client.pid sshd-process.pid; do
    if test -f "$directory/$file"; then
        kill "$(cat "$directory/$file")" 2>/dev/null || true
    fi
done
rm -rf -- "$directory"
