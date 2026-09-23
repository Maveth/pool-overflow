# Lab DATUM GW: clients.json + migrate_client

**Never deploy lab images over `bip110-datum-sv1-j` (or any live rental GW) without an explicit cutover plan.**

## Trees on NAS

| Tree | Role |
|---|---|
| `/mnt/Alexandria/local/bip110-lab/datum-luke-tmp/` | Upstream luke + migrate patches; needs ABW lab bypass (Prime configure dialect mismatch) |
| `/mnt/Alexandria/local/bip110-lab/convoy-lab-migrate/` | **Preferred lab** — copy of convoy-pr10-cbreuse-renotify + migrate patches (same lineage as prod GW image) |

Lab image tags: `datum-luke-lab:migrate`, `datum-convoy-lab:migrate`,
`datum_gateway:convoy-pr10-migrate` (M/N).

| Deploy | Stratum | API | Door |
|---|---|---|---|
| lab container | **23499** | **7199** | lab only |
| **M** `bip110-datum-sv1-m` | **23449** | **7166** | **29509** |
| **N** `bip110-datum-sv1-n` | **23451** | **7171** | **29510** |

Select/migrate by **payout address** + door registry — see `docs/MIGRATE_BY_ADDRESS.md`.  
Sink (fake peer): **29792**.

## APIs added (lab)

| API | Purpose |
|---|---|
| `GET /clients.json` | Digest-auth JSON list (`tid`, `cid`, `unique_id`, `connect_tsms`, `username`, …) |
| `POST /cmd` `migrate_client` | Body: `{cmd, tid, cid, host, port, password}` → `client.show_message` + `client.reconnect`, force-flush `w_buffer`, then kill |

Luke / convoy **`0xA4`** remains **whole-GW DATUM uplink** migrate — not per stratum client.

## Implementation notes (learned in smoke)

1. `migrate_client` must live under `case 'm':` (not nested under `case 'k':` / kill).
2. `datum_socket_send_string_to_client` only queues `w_buffer`; the normal send loop runs **after** kill closes the fd — force `send()` before kill so reconnect can land.
3. luke-tmp vs local TIDES Prime: configure can fail (`Invalid data structure`); lab used `pooled_mining_only=false` + local-GBT-without-ABW bypass so stratum could listen. Convoy-pr10 lab should speak Prime correctly (no bypass needed if configure succeeds).

## Smoke (lab only)

```text
lab SV1 client → :23499
GET /clients.json on :7199
POST migrate_client → 127.0.0.1:29792
expect: show_message + client.reconnect + kill; clients list empty
```

Script on NAS: `/tmp/_tmp_lab_migrate_smoke.py`

## Relation to pool-overflow

`overflow-agent` decides who should move; lab GW executes `migrate_client`.  
Later: pool subcomponent + optional Prime `0xA4` for whole-uplink moves.

See also `docs/GATEWAY_SPIKE.md` and NAS `LAB_MIGRATE_CLIENT.md` in each lab tree.
