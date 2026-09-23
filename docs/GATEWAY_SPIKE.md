# Gateway spike: list clients + migrate (2026-09-23)

## Live GW J (read-only)

- API: `api.listen_port` **7165** (digest admin)
- `GET /clients` works (HTML dashboard)
- Observed **3** kick-targets (`tid/cid`) and **2** unique usernames
  (`…Maveth`, `…Maveth_rental`) — matches “1 local + 2 rentals”
- Existing admin cmds: **`kill_client` / empty_thread only** — i.e. **kick**, not migrate
- `modify_conf`: false on this unit
- **No JSON list API** (`/api`, `/cmd` GET → 404); list is HTML scrape today

**Do not call kill on rental sessions.**

## Luke `0xA4` migration (datum-luke-tmp on NAS)

Implemented as `datum_protocol_migration_request` / `case 0xA4`.

**Semantics:** redirects the gateway’s **DATUM pool uplink** (host/port/pubkey)
for a time window, then return-home — **whole-GW**, not per stratum client.

So `0xA4` is perfect for “this gateway temporarily peers with Prime B”
(maintenance / house overflow at GW granularity). It is **not** “move client #3
of 3 to another pool.”

## Implications for pool-overflow

| Goal | Mechanism |
|---|---|
| See clients | Stage 1: parse `/clients` HTML → JSON (read-only). Later: native GW JSON API. |
| Move **one** SV1 session | Overflow-agent line-pump **or** new GW cmd `migrate_client` (upgrade). |
| Move **whole GW** DATUM home | Luke **`0xA4`** (when enabled + followed). |
| Tell Prime “move X” | Needs Prime+GW support for **per-session** migrate; today Prime `0xA4` is uplink-level. |

## Recommended gateway upgrade (pool subcomponent later)

1. `GET /clients.json` — tid, cid, unique_id, connect_tsms, username, hashrate  
2. `POST /cmd` `{cmd: migrate_client, tid, cid, host, port, password}`  
   — reconnect hint + kill (Stage-1); true stay-connected move later  
3. Keep `0xA4` for uplink failover / temporary peer Prime  

## Lab status (2026-09-23)

- **Done on lab GW** (not on rental J): `/clients.json` + `migrate_client` smoked PASS
  on `datum-convoy-lab-migrate` (`:23499` / `:7199` → sink `:29792`).
- Preferred lab tree: NAS `convoy-lab-migrate` (convoy-pr10 lineage; Prime configure OK).
- luke-tmp also smoked earlier but needs ABW/local-GBT bypass vs Prime dialect.
- **M / N twins** (migrate image): M `:23449`/API `:7166`/door `:29509`; N `:23451`/API `:7171`/door `:29510`.
- **Revised select flow:** match by **payout address** (not `rem_host`); reconnect via **door registry**.
  See `docs/MIGRATE_BY_ADDRESS.md` + `scripts/migrate_by_address.py` + `doors.example.toml`.
- **Do not** deploy over `bip110-datum-sv1-j` while rentals are live.

See `docs/LUKE_GW_LAB.md`.
