# E2EE device migration for OAuth-only accounts

`matrix-bridge` is an end-to-end-encrypted Matrix client. Encrypted rooms
only decrypt if the bridge (a) has its own **device identity** that the
homeserver accepts, and (b) other members' clients **share room keys** with
that device. This document covers how to get there when the normal
`matrix-bridge setup` password flow is unavailable (e.g. the account logs in
via Google/GitHub SSO and has **no password**), and how to recover when an
existing device was poisoned.

## Why the bridge needs its OWN device

A Matrix device is one logical client. Two live clients must never share a
device ID: both upload *different* identity keys claiming the same device,
which produces

- `M_UNKNOWN: One or more keys are invalid` on `/keys/upload` (400
  "device_id does not match" in some server versions),
- `Duplicate one-time keys have been uploaded` when both clients rotate
  one-time keys, and
- confusing `Sync failed: error decoding response body` sync errors.

Symptom in the bridge: every encrypted message shows
`[encrypted — unable to decrypt]` because no sender ever shares keys with a
device whose keys are invalid, and/or the to-device queue meant for the
bridge is consumed by the other client sharing the device.

**Rule:** one token = one client. If you copied the access token out of a
browser session that stays logged in, you are sharing a device — don't.

## Fresh device via SSO (`login-token`) — recommended path

New subcommand: `matrix-bridge login-token <loginToken>` performs an
`m.login.token` login (the same flow Element's "Sign in with SSO" uses) and
creates a brand-new device owned exclusively by the bridge.

### 1. Get a login token

A login token is single-use and short-lived (minutes). The helper script
`scripts/sso-login-token.py` runs a local listener and opens your browser:

```sh
python3 scripts/sso-login-token.py
# opens: https://<homeserver>/_matrix/client/v3/login/sso/redirect?redirectUrl=http://localhost:8765/callback
# log in with your SSO provider in the browser window that opens
# the script prints: LOGIN TOKEN: <token>
```

For matrix.org the homeserver base URL is `https://matrix-client.matrix.org`.
Any client-server base URL works — point it at your own server with
`--homeserver`.

### 2. Log the bridge in with it

```sh
matrix-bridge login-token '<token from step 1>'
```

This creates a new device (printed on success), saves the credentials to the
store, and does an initial sync. Afterwards:

```sh
matrix-bridge whoami          # confirm the user + new device id
matrix-bridge rooms           # confirm sync works
```

### 3. Prove decryption

In any encrypted room, post a fresh message from a **different** client
(your browser). Then:

```sh
matrix-bridge read --room '!room:id' --limit 5
```

The new message must appear as plaintext. (Older messages encrypted to dead
devices stay locked — see "History backfill" below.)

## If you already have a broken device

### Option A — full reset (simple, recommended)

```sh
mv ~/.matrix-bridge/store ~/.matrix-bridge/store.bak.$(date +%F)
matrix-bridge login-token '<fresh sso token>'   # new, clean device
```

Your config file (`~/.matrix-bridge/config.json`) is outside the store and
survives; re-set `default_room` afterwards if needed.

### Option B — surgical repair (advanced)

If you cannot create a new device (e.g. you must keep an existing device ID
because others already verified it), the device ID lives in two places and
must match:

1. `~/.matrix-bridge/store/credentials.json` — field `device_id`
2. inside the msgpack-pickled Olm account in the crypto store
   (`matrix-sdk-crypto.sqlite3`, table `account`, pickled with the SDK's
   msgpack arrangement)

`scripts/fix-device-id.py` patches both. **Back up the store first.** Only
use this when you know exactly which device ID the account pickle should
carry (e.g. you previously restored from a token and pasted a placeholder).

After any repair, verify the server agrees with what the bridge uploads:

```sh
TOKEN=$(python3 -c "import json;print(json.load(open('~/.matrix-bridge/store/credentials.json'))['access_token'])")
curl -s -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json' \
  -d '{"device_keys":{"@you:server":["DEVICEID"]}}' \
  https://matrix.org/_matrix/client/v3/keys/query | python3 -m json.tool
```

The returned device keys for that device must match the keys in
`~/.matrix-bridge/store/matrix-sdk-crypto.sqlite3`
(`identities` table, base64 in the pickle) — if the server's copy is
different, another client is still sharing the device.

## History backfill

Messages encrypted to *destroyed* keys are not recoverable by any client
trick. Your options:

- **Server-side key backup**: `matrix-bridge restore --recovery-key-file f
  --version N` (needs the backup's recovery key; note that the newer
  `0x8B 0x01`-format recovery keys are rejected by matrix-sdk-crypto 0.14's
  base58 parser as of this writing — export/import below is more reliable)
- **Element "Export E2E room keys"** (Settings → Security & Privacy) produces
  a passphrase-encrypted JSON file; an importer for that format is not yet
  included — contributions welcome.

## Trust modes

`trust_mode` in the config controls how device keys are trusted (`tofu` =
trust on first use). With default Element settings, unverified devices still
receive keys, so red locks on the sender side don't block decryption; see
[e2ee-session-verification.md](e2ee-session-verification.md) for making the
bridge session verified.
