#!/usr/bin/env bash
# Migrate matrix-bridge onto a device created in a throwaway browser session.
#
# WHY: if your account has no password (SSO/OAuth only), `matrix-bridge setup`
# can't log in. Instead: open a PRIVATE window, log in via SSO, copy the
# Access Token (Element: Settings -> Help & About -> Access Token), and run:
#
#   scripts/migrate-bridge.sh 'syt_...' [--refuse-device OLDDEVICE]
#
# The --refuse-device guard stops you from (again) handing over the token of
# a browser session that STAYS LOGGED IN — sharing a live device between two
# clients breaks both (see docs/e2ee-device-migration.md).
#
# After a successful run, CLOSE THE PRIVATE WINDOW WITHOUT SIGNING OUT
# (signing out would delete the device the bridge just adopted... which is
# fine too — the bridge only needs the token, not the browser session).
set -euo pipefail

HOMESERVER="${MATRIX_HOMESERVER:-https://matrix.org}"
TOKEN="${1:?usage: migrate-bridge.sh '<ACCESS_TOKEN>' [refuse-device]}"
shift || true
REFUSE=""

while [ $# -gt 0 ]; do
  case "$1" in
    --refuse-device) REFUSE="${2:?--refuse-device needs an argument}"; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 1 ;;
  esac
done

if ! command -v matrix-bridge >/dev/null 2>&1; then
  echo "matrix-bridge not on PATH; set it up or use the full path to the binary." >&2
  exit 1
fi

echo "Checking token with $HOMESERVER..."
WHOAMI=$(curl -sf -H "Authorization: Bearer $TOKEN" \
  "$HOMESERVER/_matrix/client/v3/account/whoami") || {
  echo "Token invalid or network error — nothing was changed." >&2
  exit 1
}
USER_ID=$(echo "$WHOAMI" | python3 -c "import sys,json;print(json.load(sys.stdin)['user_id'])")
DEVICE_ID=$(echo "$WHOAMI" | python3 -c "import sys,json;print(json.load(sys.stdin)['device_id'])")
echo "Token OK: user=$USER_ID device=$DEVICE_ID"

if [ -n "$REFUSE" ] && [ "$DEVICE_ID" = "$REFUSE" ]; then
  echo "Refusing: this token belongs to device $REFUSE (see --refuse-device)." >&2
  echo "Copy the token from the NEW throwaway session instead." >&2
  exit 1
fi

STORE="${MATRIX_BRIDGE_STORE:-$HOME/.matrix-bridge/store}"
echo "Backing up and wiping old bridge store at $STORE ..."
mkdir -p "$STORE"
tar czf "$STORE.bak.$(date +%Y%m%d-%H%M%S).tar.gz" -C "$(dirname "$STORE")" "$(basename "$STORE")"
rm -f "$STORE"/matrix-sdk-*.sqlite3* "$STORE"/credentials.json

echo "Registering device with the bridge..."
matrix-bridge restore-token "$USER_ID" "$TOKEN" "$DEVICE_ID"

cat <<EOF

Done — the bridge is now its own device: $DEVICE_ID
Close the private window WITHOUT signing out.
Then post a fresh message in an encrypted room from your normal client and:
  matrix-bridge read --room '!room:id' --limit 5
...to confirm decryption.
EOF
