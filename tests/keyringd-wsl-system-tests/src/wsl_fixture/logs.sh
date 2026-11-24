set -euo pipefail
directory=$1
export XDG_RUNTIME_DIR=/run/user/0
for name in setup-stage sshd.log socket-before-connect; do
    printf '\n=== %s ===\n' "$name"
    if test -f "$directory/$name"; then
        cat "$directory/$name"
    else
        printf '%s\n' 'not created'
    fi
done
for name in sshd-process; do
    if test -f "$directory/$name.pid"; then
        pid=$(cat "$directory/$name.pid")
        printf '\n=== %s process ===\n' "$name"
        ps -p "$pid" -o pid=,stat=,args= || true
    fi
done
printf '\n=== proxy socket ===\n'
if test -S /run/user/0/keyringd-agent.sock; then
    printf '%s\n' present
else
    printf '%s\n' absent
fi
systemctl --user show keyringd.service --property=MainPID --property=ActiveState
journalctl --user -u keyringd.service --no-pager -n 40
