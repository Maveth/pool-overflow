#!/usr/bin/env python3
"""Read-only DATUM GW /clients enumerator → JSON.

Does NOT call kill_client / migrate. Safe to run against live rental GWs.

Example:
  python3 scripts/gw_clients_enum.py \\
    --url http://127.0.0.1:7165 --user admin --password-file /path/to/pw
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

import requests
from requests.auth import HTTPDigestAuth


def redact_user(u: str) -> str:
    if "." in u:
        a, w = u.split(".", 1)
        return f"{a[:8]}…{a[-4:]}.{w}"
    if len(u) > 12:
        return f"{u[:8]}…{u[-4:]}"
    return u


def parse_clients_html(html: str) -> list[dict]:
    """Best-effort parse of datum_gateway /clients HTML table."""
    out: list[dict] = []
    # Kick buttons encode tid_cid_connectTs_uniqueId
    kills = re.findall(
        r"kill_client['\"]?\s*,\s*tid:(\d+)\s*,\s*cid:(\d+)\s*,\s*t:(\d+)\s*,\s*id:(\d+)",
        html,
    )
    if not kills:
        kills = re.findall(r"value='(\d+)_(\d+)_(\d+)_(\d+)'", html)
    users = re.findall(
        r"(bc1[qpzry9a-z0-9]{20,}(?:\.[A-Za-z0-9_.:-]*)?)", html, flags=re.I
    )
    # Pair by order when counts match; else list separately
    for i, k in enumerate(kills):
        row = {
            "tid": int(k[0]),
            "cid": int(k[1]),
            "connect_tsms": int(k[2]),
            "unique_id": int(k[3]),
            "username": users[i] if i < len(users) else None,
            "username_redacted": redact_user(users[i]) if i < len(users) else None,
        }
        out.append(row)
    if len(users) > len(kills):
        for u in users[len(kills) :]:
            out.append(
                {
                    "tid": None,
                    "cid": None,
                    "username": u,
                    "username_redacted": redact_user(u),
                }
            )
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--url", required=True, help="e.g. http://127.0.0.1:7165")
    ap.add_argument("--user", default="admin")
    ap.add_argument("--password", default="")
    ap.add_argument("--password-file", type=Path)
    ap.add_argument("--raw-html-out", type=Path, help="optional debug dump")
    args = ap.parse_args()
    pw = args.password
    if args.password_file:
        pw = args.password_file.read_text(encoding="utf-8").strip()

    base = args.url.rstrip("/")
    r = requests.get(
        f"{base}/clients", auth=HTTPDigestAuth(args.user, pw), timeout=10
    )
    if r.status_code != 200:
        r = requests.get(f"{base}/clients", auth=(args.user, pw), timeout=10)
    if r.status_code != 200:
        print(json.dumps({"ok": False, "status": r.status_code}), file=sys.stderr)
        return 1
    if args.raw_html_out:
        args.raw_html_out.write_text(r.text, encoding="utf-8")

    clients = parse_clients_html(r.text)
    print(
        json.dumps(
            {
                "ok": True,
                "url": f"{base}/clients",
                "count": len(clients),
                "clients": clients,
                "note": "READ-ONLY. Existing GW API can kill_client only — no migrate_client yet. Luke 0xA4 moves whole DATUM uplink, not per SV1 session.",
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
