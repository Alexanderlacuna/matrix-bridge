#!/usr/bin/env python3
"""Surgically change the device id inside matrix-bridge's crypto store.

USE ONLY when you must keep an existing device ID — e.g. you restored the
bridge from a token and pasted a placeholder like "<device-id>" into
credentials.json, so the bridge's uploaded keys are rejected with
"device_id does not match" and nobody ever shares keys with it.

The device id lives in TWO places and they must agree:
  1. ~/.matrix-bridge/store/credentials.json          (field "device_id")
  2. the msgpack-pickled Olm account inside
     ~/.matrix-bridge/store/matrix-sdk-crypto.sqlite3  (table "account")

This script patches the pickle with a same-length byte replacement
(re-serializing the msgpack would change rmp_serde's layout and corrupt
the account).

Prereqs: the OLD id must appear EXACTLY once in the pickle, and the NEW id
must have the SAME length. Back up the store first (the script aborts
without --write and prints where the backup should go).

Usage:
    python3 fix-device-id.py --old '<OLD_ID>' --new '<NEW_ID>' [--write]
    python3 fix-device-id.py --old '<OLD_ID>' --new '<NEW_ID>' \
        --store /path/to/store
"""
import argparse
import json
import shutil
import sqlite3
import sys
from pathlib import Path


def fail(msg: str) -> None:
    print(f"ERROR: {msg}", file=sys.stderr)
    sys.exit(1)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--old", required=True, help="device id currently in the pickle")
    ap.add_argument("--new", required=True, help="device id that must be there instead")
    ap.add_argument("--store", default=str(Path.home() / ".matrix-bridge" / "store"))
    ap.add_argument("--write", action="store_true",
                    help="actually modify the store (default: dry run)")
    args = ap.parse_args()

    if len(args.old) != len(args.new):
        fail(f"length mismatch: {args.old!r} vs {args.new!r} — same length required")

    store = Path(args.store)
    cred_path = store / "credentials.json"
    db_path = next(store.glob("matrix-sdk-crypto*.sqlite3"), None)
    if cred_path is None or not cred_path.exists():
        fail(f"no credentials.json under {store}")
    if db_path is None:
        fail(f"no matrix-sdk-crypto sqlite under {store}")

    creds = json.loads(cred_path.read_text())
    cred_device = creds.get("device_id")
    print(f"credentials.json device_id : {cred_device!r}")

    con = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
    rows = con.execute("SELECT id, data FROM account").fetchall()
    con.close()
    if not rows:
        fail("account table is empty — nothing to patch")
    row_id, blob = rows[0]

    occurrences = blob.count(args.old.encode())
    print(f"pickle contains {args.old!r}: {occurrences}x")
    if occurrences != 1:
        fail("expected exactly one occurrence; refusing to guess")
    patched = blob.replace(args.old.encode(), args.new.encode())

    if not args.write:
        out = store / "account.patched.bin"
        out.write_bytes(patched)
        print(f"dry run OK — patched pickle written to {out}")
        print("re-run with --write to update the sqlite store "
              "(back it up first: cp -a the store directory)")
        return

    backup = store.with_name(store.name + f".bak-before-fix")
    if not backup.exists():
        shutil.copytree(store, backup)
        print(f"backup: {backup}")
    con = sqlite3.connect(db_path)
    con.execute("UPDATE account SET data = ? WHERE id = ?", (patched, row_id))
    con.commit()
    con.close()
    print("store patched.")

    creds["device_id"] = args.new
    cred_path.write_text(json.dumps(creds, indent=2) + "\n")
    print("credentials.json updated.")
    print()
    print("Next: start the bridge, check `matrix-bridge whoami`, then confirm")
    print("the server now publishes the same keys the bridge uploads (keys/query).")


if __name__ == "__main__":
    main()
