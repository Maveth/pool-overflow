# pool-overflow

Pool-side **SV1 / DATUM overflow valve** for BIP-110 pools.

## For coders

Miners still connect to a normal pool. That pool runs `overflow-agent`, which may
keep the session local or **line-pump** SV1 to another destination when house
policy says “too much SV1 / this address is overflow-eligible.” Destinations pay
under the miner’s own username (Lazarus-style).  

An optional **`federation-directory`** only tracks peers, rates, and criteria —
the agent works from a local `peers.toml` with **no directory at all**.

DATUM sessions should eventually use Luke’s **`0xA4` GWMigration** (mine on B /
return home) per gateway session; Stage 1 focuses on SV1 pump and leaves `0xA4`
as an explicit next hook.

See [DESIGN.md](DESIGN.md).

## Workspace

| Crate | Role |
|---|---|
| `overflow-agent` | Stage 1 valve: authorize-peek, lease, opaque pump, local peers |
| `federation-directory` | Optional HTTP registry for pool rates / peer list |
| `overflow-core` | Shared types (peer, lease, report, criteria) |

## Build

```bash
cargo build --workspace --release -j 1
```

## Status

- [x] Design: pool-local first, federation optional  
- [x] Core types + directory HTTP stub  
- [x] Agent SV1 authorize-peek + opaque pump + sticky leases  
- [x] Address class: pinned / known / new (+ pin API)  
- [x] Agent status HTTP (`:29791` in example)  
- [x] Light TCP health cache (30s TTL — gentle on live J)  
- [x] `overflow-sink` lab destination for dual-path tests  
- [ ] Auto house-share meter (still operator-supplied pct)  
- [ ] Failover rebind when backend goes unhealthy mid-lease  
- [x] Spike: GW `/clients` enum (read-only) + `0xA4` = whole-uplink not per-client  
- [x] Lab GW `clients.json` + `migrate_client` (M/N twins; not on rental J)  
- [x] Address-first migrate helper + door registry (`scripts/migrate_by_address.py`)  
- [x] Shareable GW feature: [datum_gateway-migrate-pr#1](https://github.com/Maveth/datum_gateway-migrate-pr/pull/1)  
- [x] Agent `POST /api/gw/migrate` (doors + payout address)  
- [ ] Auto policy: decide overflow then call GW migrate (not only SV1 pump)  
- [ ] Directory pull of live peer rates  

### Lab ports (do not use HAP)

| Service | Port |
|---|---|
| overflow-agent SV1 | `0.0.0.0:29790` |
| overflow-agent HTTP | `0.0.0.0:29791` |
| overflow-sink (fake pool) | `0.0.0.0:29792` |
| federation-directory | `0.0.0.0:29880` |
| Keep-local backend | `127.0.0.1:23446` (GW J internal — rentals may be live; be gentle) |

Dual-path lab: `agent-dual-path.example.toml` (house share 20% → overflow to sink).
