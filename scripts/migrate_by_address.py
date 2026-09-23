#!/usr/bin/env python3
"""Select DATUM GW sessions by payout identity and migrate them to a door.

Revised flow (HAP/socat agnostic):
  1. Pick source GW (its admin API)
  2. GET /clients.json
  3. Match username by payout address (exact worker OR address.* prefix)
  4. POST migrate_client for each match → destination door host:port
     from doors.toml (never from rem_host)

Examples:
  # dry-run: list matches on N
  python3 scripts/migrate_by_address.py \\
    --doors doors.example.toml --from N --identity 'bc1q….Maveth' --dry-run

  # migrate all workers on address to M door
  python3 scripts/migrate_by_address.py \\
    --doors doors.toml --from N --to M \\
    --identity bc1qj30fwc353ketu3nwm0lq0gzmmygu4qn4nh5h8m --match address \\
    --password-file /path/to/admin.pw

Never point this at rental sessions on J unless explicitly intended.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # py<3.11
    import tomli as tomllib  # type: ignore


def normalize_address(username: str) -> str:
    """Strip worker suffix: bc1q….worker → bc1q…"""
    u = (username or "").strip()
    if "." in u:
        return u.split(".", 1)[0]
    return u


def matches(username: str, identity: str, mode: str) -> bool:
    u = (username or "").strip()
    ident = identity.strip()
    if mode == "exact":
        return u == ident
    if mode == "address":
        # identity may be address or address.worker — compare normalized
        return normalize_address(u).lower() == normalize_address(ident).lower()
    if mode == "prefix":
        # address.* style: username startswith address.
        addr = normalize_address(ident)
        return u == addr or u.startswith(addr + ".")
    raise ValueError(f"unknown match mode {mode}")


def load_doors(path: Path) -> dict:
    raw = tomllib.loads(path.read_text(encoding="utf-8"))
    doors = raw.get("doors") or {}
    if not doors:
        raise SystemExit(f"no [doors.*] in {path}")
    return doors


def curl_json(
    url: str,
    *,
    password: str,
    data: dict | None = None,
    timeout: int = 15,
) -> tuple[int, str]:
    if data is None:
        cmd = [
            "curl",
            "-sS",
            "-m",
            str(timeout),
            "--digest",
            "-u",
            f"admin:{password}",
            "-w",
            "\n__HTTP__%{http_code}",
            url,
        ]
    else:
        payload = dict(data)
        payload.setdefault("password", password)
        cmd = [
            "curl",
            "-sS",
            "-m",
            str(timeout),
            "-H",
            "Content-Type: application/json",
            "-d",
            json.dumps(payload),
            "-w",
            "\n__HTTP__%{http_code}",
            url,
        ]
    r = subprocess.run(cmd, capture_output=True, text=True)
    out = r.stdout
    if "__HTTP__" not in out:
        return r.returncode or 1, (r.stderr or out)
    body, _, code = out.rpartition("__HTTP__")
    return int(code.strip() or "0"), body


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--doors", type=Path, required=True)
    ap.add_argument("--from", dest="src", required=True, help="source door id, e.g. N")
    ap.add_argument("--to", dest="dst", help="dest door id, e.g. M (required unless --dry-run)")
    ap.add_argument("--identity", required=True, help="payout address or address.worker")
    ap.add_argument(
        "--match",
        choices=("exact", "address", "prefix"),
        default="exact",
        help="exact=full username; address/prefix=all workers on payout (address.*)",
    )
    ap.add_argument("--password", default="")
    ap.add_argument("--password-file", type=Path)
    ap.add_argument(
        "--password-from-gw-config",
        type=Path,
        help="read api.admin_password from a GW config.json",
    )
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    doors = load_doors(args.doors)
    if args.src not in doors:
        raise SystemExit(f"unknown --from {args.src}; have {sorted(doors)}")
    if not args.dry_run and not args.dst:
        raise SystemExit("--to required unless --dry-run")
    if args.dst and args.dst not in doors:
        raise SystemExit(f"unknown --to {args.dst}; have {sorted(doors)}")

    pw = args.password
    if args.password_file:
        pw = args.password_file.read_text(encoding="utf-8").strip()
    if args.password_from_gw_config:
        cfg = json.loads(args.password_from_gw_config.read_text(encoding="utf-8"))
        pw = cfg["api"]["admin_password"]
    if not pw:
        raise SystemExit("need --password / --password-file / --password-from-gw-config")

    src = doors[args.src]
    api = str(src["api"]).rstrip("/")
    code, body = curl_json(f"{api}/clients.json", password=pw)
    if code != 200:
        print(f"FAIL clients.json HTTP {code}: {body[:300]}", file=sys.stderr)
        return 2
    data = json.loads(body)
    clients = data.get("clients") or []
    hits = [c for c in clients if matches(c.get("username") or "", args.identity, args.match)]
    print(
        json.dumps(
            {
                "src": args.src,
                "identity": args.identity,
                "match": args.match,
                "normalized": normalize_address(args.identity),
                "total_on_gw": len(clients),
                "matched": [
                    {
                        "tid": c.get("tid"),
                        "cid": c.get("cid"),
                        "username": c.get("username"),
                        "rem_host_ignored": c.get("rem_host"),
                    }
                    for c in hits
                ],
            },
            indent=2,
        )
    )
    if not hits:
        print("no matching sessions", file=sys.stderr)
        return 3
    if args.dry_run:
        print("dry-run: no migrate")
        return 0

    dst = doors[args.dst]
    host = str(dst["host"])
    port = int(dst["port"])
    results = []
    for c in hits:
        tid, cid = int(c["tid"]), int(c["cid"])
        code, body = curl_json(
            f"{api}/cmd",
            password=pw,
            data={
                "cmd": "migrate_client",
                "tid": tid,
                "cid": cid,
                "host": host,
                "port": port,
            },
        )
        results.append({"tid": tid, "cid": cid, "http": code, "body": body.strip()[:200]})
        print(f"migrate {tid}/{cid} -> {host}:{port} HTTP {code} {body.strip()[:120]}")
    ok = all(r["http"] == 200 for r in results)
    return 0 if ok else 4


if __name__ == "__main__":
    raise SystemExit(main())
