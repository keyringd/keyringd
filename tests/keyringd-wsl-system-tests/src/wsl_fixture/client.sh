set -euo pipefail
directory=$1
trap 'rm -f -- "$directory/client.pid"' EXIT
timeout 45 ssh -F "$directory/ssh_config" -v fixture 'printf keyringd-wsl-ok' &
client=$!
echo "$client" > "$directory/client.pid"
wait "$client"
