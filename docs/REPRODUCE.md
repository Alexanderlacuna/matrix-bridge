# Reproduction guide: zero → reading E2EE room history

Ordered steps from a fresh machine to a working read-only MCP bridge with
historical message decryption. Each step links the doc with details.

## 1. Build

```sh
git clone https://github.com/Alexanderlacuna/matrix-bridge
cd matrix-bridge
# MSRV needs a recent toolchain; a fully populated cargo cache allows:
cargo +1.88.0 build --release --offline
# binaries: target/release/matrix-bridge, matrix-bridge-mcp
```

## 2. Configure

`~/.matrix-bridge/config.json`:

```json
{
  "homeserver": "https://matrix.org",
  "username": "@you:matrix.org",
  "default_room": "!roomid:matrix.org",
  "trust_mode": "tofu"
}
```

## 3. Log in (passwordless / SSO accounts)

`setup` needs a password. For OAuth-only accounts use the SSO login-token
flow so the bridge gets its **own** device — never reuse a browser session's
access token (one token = one client; sharing breaks both):

```sh
python3 scripts/sso-login-token.py          # prints a local URL, captures the token
matrix-bridge login-token <loginToken>      # creates device, saves credentials
```

Details: [e2ee-device-migration.md](e2ee-device-migration.md). Password
accounts can just run `matrix-bridge setup`.

## 4. Smoke test

```sh
matrix-bridge rooms
matrix-bridge read --room '!roomid:matrix.org' --limit 10
```

Messages from after the bridge device existed decrypt. Older ones show
`[encrypted — unable to decrypt]` until step 6.

## 5. Wire the MCP (read-only)

`~/.mcp.json` (also copy into your project's `.mcp.json`):

```json
{
  "mcpServers": {
    "matrix-element": {
      "command": "/abs/path/to/matrix-bridge-mcp",
      "excludeTools": ["send_message", "send_and_wait", "join_room"],
      "lifecycle": "lazy",
      "directTools": true
    }
  }
}
```

**Restart the agent session** (Emacs/Pi) — MCP servers are loaded at
startup. Then `read_messages` on `matrix-element` works.

## 6. Unlock pre-bridge history (key export import)

Element (the session that can read the room): **Settings → Encryption →
Export E2E room keys**, set a passphrase, save the downloaded file. Then,
with **no other bridge process running**:

```sh
matrix-bridge import-export --file ~/Downloads/element-keys.txt \
    --passphrase '...'
matrix-bridge read --room '!roomid:matrix.org' --limit 50
```

Gotchas: use the downloaded file, not a copy-paste (pastes get truncated);
format spec in [e2ee-key-export-import.md](e2ee-key-export-import.md).
Delete or securely store the export file afterwards — it holds every room
key. Messages from sessions the exporting device never held stay locked.

## 7. Optional: verify the bridge session

```sh
matrix-bridge verify-wait --timeout 600
```

Then in Element: verify the new session, emoji match. If a previous attempt
failed, **clear Element's cache and reload first** — failed verification
dialogs go stale and never re-send. Details:
[e2ee-session-verification.md](e2ee-session-verification.md).

## 8. Cleanup

- Remove dead devices (old bridge attempts) in Element → Settings → Sessions.
- Keep `~/.matrix-bridge/store/` backed up; it holds all keys.
