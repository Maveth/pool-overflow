# Pool overflow + optional federation directory

## Thoughts (architecture choice)

**Yes — pool-as-node is the right Stage 1.**

Miners keep connecting to *a pool*. That pool’s SV1/DATUM stack decides whether a
session stays local or is sent elsewhere. The product works **with zero
federation**: one operator runs an overflow valve (Lazarus-style) against a
hand-edited peer list.

**Federation is only the shared backend** for:

- curated peer list / pubkeys  
- self-reported rates + soft network scores  
- max fee / eligibility criteria  
- optional “known address” hints  

Not a miner-facing “mesh hostname,” not autonomous peer discovery.

```text
                    ┌─────────────────────────┐
   optional feed    │  federation-directory   │  (rates, peers, criteria)
                    └───────────┬─────────────┘
                                │ pull / push
   ASIC ──SV1──► Pool A valve ──┼── keep local
                 (overflow-agent)│
                                 └── overflow ──► Pool B / C / …
                                       │
                    DATUM GW ◄── 0xA4 migrate (when implemented)
```

## Stage 1 — `overflow-agent` (works alone)

Runs beside a pool’s SV1 (and later DATUM GW):

1. On new SV1 connect: buffer until `mining.authorize` → payout address.  
2. Classify address: **known** (recent shares / coinbase / local memory) vs **new**.  
3. If over policy (e.g. house SV1 % too high) and address is overflow-eligible →  
   **opaque line-pump** to a destination from the peer list (Lazarus pattern).  
4. Else keep local (or fail-open local).  
5. Sticky lease so the same address doesn’t hop every reconnect.  
6. Prefer destinations with `fee_bps ≤ max_fee_bps`.

**No federation required** — `peers.toml` on disk is enough.

## Federation directory (thin, parallel)

Separate small service / crate:

- Stores peer pool records (id, pubkey, SV1 host:port, fee, self-reported HR, tag reliability).  
- Serves JSON for agents to pull.  
- Later: signed updates from pool operators.  

Agents that never talk to the directory still work.

## DATUM `0xA4` GWMigration (spike 2026-09-23)

See `docs/GATEWAY_SPIKE.md`. Luke’s **`0xA4`** (NAS `datum-luke-tmp`) redirects the
gateway’s **whole DATUM uplink** (host/port/pubkey) for a deadline, then
return-home — **not** per stratum client.

| Path | Mechanism |
|---|---|
| Per SV1 session | Authorize-peek + line-pump **or** future GW `migrate_client` |
| Whole-GW DATUM home | **`0xA4`** uplink migrate (when enabled) |

Stock GW admin: `/clients` HTML + **`kill_client` only**. Read-only enum:
`scripts/gw_clients_enum.py`. Agent decides; GW/Prime execute when APIs exist.

## Address “newness”

Soft signals (combine, don’t overfit):

- Seen in recent accepted-share / TIDES window  
- Seen in recent coinbase outputs for this pool  
- Explicit friend-pin / prefer list (counts against local pie)

**New** → easier to overflow. **Known / pinned** → stay.

## Non-goals

- Banning SV1  
- Silent brand steal without disclosure  
- Self-discovering mesh protocol  
- Touching Riptide HAP / production LBs from this repo’s lab defaults  

## Relation to earlier repos

| Repo | Role now |
|---|---|
| `sv1-mesh` (private) | Early L4 experiment |
| `sv1-federation` | Neutral entry + pump experiment — useful code to steal |
| **`pool-overflow` (this)** | Product direction: pool-local valve + optional directory |
