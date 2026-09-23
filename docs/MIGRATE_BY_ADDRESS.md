# Migrate by payout address (revised flow)

## Problem

All miners reach GWs through **HAP or socat**. On the gateway,
`rem_host` is almost always `127.0.0.1` / `::ffff:127.0.0.1`. It cannot
identify a miner or supply a `client.reconnect` host.

## Revised control flow

```
pick source GW
  → GET /clients.json
  → match sessions by payout identity
  → for each tid/cid: POST migrate_client → door[dest].host:port
```

| Concept | Source of truth |
|---|---|
| Who to move | **payout address** (username from authorize) |
| Session scalpel | `tid` / `cid` on that GW only |
| Reconnect target | **door registry** (`doors.toml`), not `rem_host` |

## Match modes

| Mode | Meaning |
|---|---|
| `exact` | full `address.worker` (lab / single worker) |
| `address` / `prefix` | **`address.*`** — every worker on that payout |

**Later product:** rate math, sticky lease (~24h), and overflow decisions key on
**normalized address** (strip `.worker`). Worker is display / ops only.

## Lab doors (Maveth twins)

| Id | Internal stratum | API | Advertised door |
|---|---|---|---|
| M | 23449 | 7166 | 29509 |
| N | 23451 | 7171 | 29510 |

Image: `datum_gateway:convoy-pr10-migrate` (PR10 lineage + `clients.json` /
`migrate_client`). **Not** deployed on rental J.

## Tooling

```bash
# list matches only
python3 scripts/migrate_by_address.py \
  --doors doors.example.toml --from N \
  --identity 'bc1q….Maveth' --match exact \
  --password-from-gw-config /path/to/n/config.json \
  --dry-run

# bounce N → M by address (all workers)
python3 scripts/migrate_by_address.py \
  --doors doors.toml --from N --to M \
  --identity bc1q… --match address \
  --password-from-gw-config /path/to/n/config.json
```

See also `docs/GATEWAY_SPIKE.md`, `docs/LUKE_GW_LAB.md`.
