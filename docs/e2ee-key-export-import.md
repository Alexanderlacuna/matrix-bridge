# Importing an Element key export (pre-bridge history)

Element Web/Desktop can export all Megolm session keys it holds to a
passphrase-encrypted file. `matrix-bridge import-export` decrypts that file
and imports the sessions into the bridge's crypto store, unlocking messages
that were sent **before** the bridge device existed.

## Usage

1. In Element: **Settings → Encryption → Export E2E room keys** (the file is
   armored as `-----BEGIN MEGOLM SESSION DATA-----` and downloaded, e.g.
   `element-keys.txt`).
2. Stop anything else using the bridge store (MCP server, `verify-wait`).
3. Import:

   ```sh
   matrix-bridge import-export --file ~/Downloads/element-keys.txt
   # or non-interactively:
   matrix-bridge import-export --file ~/Downloads/element-keys.txt --passphrase '...'
   ```

4. `matrix-bridge read` again — messages covered by the export now decrypt.

## File format (v1)

Verified against Element's
[`MegolmExportEncryption.ts`](https://github.com/matrix-org/matrix-react-sdk/blob/develop/src/utils/MegolmExportEncryption.ts):

```
base64(
  0x01                 version
  salt[16]             random
  iv[16]               AES-CTR counter
  kdf_iterations[4]    big-endian u32 (usually 500000)
  ciphertext           AES-256-CTR(JSON), length = total - (1+16+16+4+32)
  hmac[32]             HMAC-SHA256(key[32..64], everything above)
)
```

Key derivation: `PBKDF2-HMAC-SHA512(passphrase, salt, kdf_iterations, 64
bytes)`; bytes `[0..32]` are the AES-256 key, bytes `[32..64]` the HMAC-SHA-256
key. Note the HMAC is **SHA-256 over the entire header + ciphertext** (not
SHA-512, not ciphertext-only) — getting this wrong fails with a checksum
mismatch even when the passphrase is right.

The decrypted JSON is either:

- a **flat array** of session objects (Rust crypto stack, current default), or
- `{"rooms": {"<room_id>": {"sessions": {"<session_id>": {...}}}}}` (legacy
  libolm stack).

Both shapes are accepted; each object deserializes into matrix-sdk-crypto's
`ExportedRoomKey` and is imported via `Store::import_exported_room_keys`.

## Limitations

- Only sessions the **exporting device** holds are included. Messages from
  sessions it never received (sender offline, key forwarding disabled, or
  sender never shared with that device) stay locked. Re-exporting later may
  pick up newly forwarded keys.
- The export file contains every room key of the exporting session. Keep it
  private; delete it once imported.
- Run the import while no other bridge process holds the crypto store.
