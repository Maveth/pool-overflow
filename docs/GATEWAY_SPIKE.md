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
2. `POST /cmd` `{cmd: migrate_client, tid, cid, dest_host, dest_port, ttl_secs}`  
   — true move, not kill  
3. Keep `0xA4` for uplink failover / temporary peer Prime  

Until (2) exists: SV1 path = authorize-peek + pump; DATUM path = whole-GW `0xA4` only.
