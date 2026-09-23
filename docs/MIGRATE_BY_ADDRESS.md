# Migrate by payout address

Federation/ops side of per-client move. The Gateway APIs themselves are
documented upstream-style in:

https://github.com/Maveth/datum_gateway-migrate-pr/blob/add-client-migrate/doc/client_migrate.md

## Flow

```
pick source GW
  → GET /clients.json
  → match sessions by payout identity
  → POST migrate_client → door[dest].host:port
```

| | Source of truth |
|--|--|
| Who | payout address (Stratum username; `address.*` = all workers) |
| Session | `tid` / `cid` on that GW |
| Where | door registry (`doors.toml`) — not `rem_host` |

## Match modes

- `exact` — full `address.worker`
- `address` — every worker on that payout

## Tools

**CLI**

```bash
python3 scripts/migrate_by_address.py \
  --doors doors.toml --from M --to N \
  --identity bc1q… --match address \
  --password-from-gw-config /path/to/gw/config.json
```

**overflow-agent** (optional)

```toml
gw_doors_path = "doors.toml"
gw_admin_password_env = "OVERFLOW_GW_ADMIN_PASSWORD"
```

```http
POST /api/gw/migrate
{"from":"M","to":"N","identity":"bc1q…","match":"address","dry_run":false}
```

## Lab doors (MaVeTh)

| Id | Stratum | API | Door |
|----|---------|-----|------|
| M | 23449 | 7166 | 29509 |
| N | 23451 | 7171 | 29510 |
