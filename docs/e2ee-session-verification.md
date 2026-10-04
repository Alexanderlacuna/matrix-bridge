# Verifying the bridge session (emoji SAS, automatic)

Element shows unverified sessions with a red warning, and some users run
"only send to verified devices" — so it's useful to make the bridge's device
verified. The bridge has no screen, so it can't show emoji. This fork adds
**auto-verify**: the bridge automatically accepts and confirms emoji
verification — but only for requests from **its own user** (self-verification
from your Element session). Requests from anyone else are ignored.

## Commands

```sh
# Arm auto-verify and keep syncing for N seconds while you click through in Element:
matrix-bridge verify-wait --timeout 300

# Check the current device id:
matrix-bridge whoami
```

## Steps

1. Run `matrix-bridge verify-wait --timeout 300` in a terminal.
2. In Element Web: **Settings → Security & Privacy → Sessions** → click the
   `matrix-bridge` device → **Verify** → *"Verify by emoji"*.
3. Seven emoji appear on your screen. The bridge has already confirmed its
   side automatically — you click the green **"They match"**.

When the flow completes, your cross-signing self-signing key signs the
bridge's device key: the session shows **Verified** everywhere, and the
verification secret is transferred to the bridge over the SAS channel.

## Verify it worked

```sh
TOKEN=$(python3 -c "import json;print(json.load(open('~/.matrix-bridge/store/credentials.json'))['access_token'])")
curl -s -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"device_keys":{"@you:server":["BRIDGE_DEVICE_ID"]}}' \
  https://matrix.org/_matrix/client/v3/keys/query | \
  python3 -c "import json,sys; d=json.load(sys.stdin); \
print(list(d['device_keys']['@you:server']['BRIDGE_DEVICE_ID']['signatures']['@you:server'].keys()))"
```

Before: only `ed25519:BRIDGE_DEVICE_ID` (self-signature).
After successful verification: additionally `ed25519:<self-signing-key-id>`.

## Known issues

- **Stuck verification dialogs.** If a previous verification attempt failed
  or was cancelled, Element Web can keep showing the *old* emoji dialog when
  you click "Verify" again, without sending a new request — the bridge then
  receives nothing and the retry fails or hangs. Fix: in Element, **avatar →
  Help & About → "Clear cache and reload"**, wait for the resync, then start
  the verification again.
- **"One of the following may be compromised…"** after clicking "They
  match" means the two sides completed *different* verification sessions —
  almost always the stale-dialog situation above. Cancel everything, reload
  the cache, retry once.
- Only **one** Element click matters: the green "They match". The bridge
  accepts the request and confirms the emoji by itself.

## Security note

Auto-accepting verification without displaying emoji removes the
man-in-the-middle check *for your own user only*. That's the same trust you
already place in your own account (any of your verified sessions could
cross-sign a new device anyway). Never enable this for other users' requests
— the handler deliberately filters `sender == own user`.
