# Lab DATUM GW fork (NAS `datum-luke-tmp`)

**Not a GitHub fork yet** — edits live under:

`/mnt/Alexandria/local/bip110-lab/datum-luke-tmp/`

**Never deploy this over `bip110-datum-sv1-j` while rentals are live.**

## Patches applied (lab)

| API | Purpose |
|---|---|
| `GET /clients.json` | Structured client list (tid/cid/unique_id/username/…) |
| `POST /cmd` `migrate_client` | Per-client: `client.reconnect` + show_message, then kill |

Luke **`0xA4`** (already in this tree) = whole-GW DATUM uplink migrate — kept as-is.

See also `LAB_MIGRATE_CLIENT.md` in that tree.

## Why reconnect+kill (for now)

True per-client DATUM “stay connected, change home” needs deeper GW work.
Stage-1 migrate uses stratum reconnect (best-effort) then kill so we never leave
a client hashing on the wrong extranonce if reconnect is ignored.

## Relation to pool-overflow

`overflow-agent` decides who should move; lab GW executes `migrate_client`.
Later: pool subcomponent + optional “tell Prime” for real `0xA4` uplink moves.
