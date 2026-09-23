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
- [x] Core types + agent/directory stubs  
- [ ] Agent SV1 peek+pump (port from `sv1-federation`)  
- [ ] Known-address heuristic hooks  
- [ ] DATUM `0xA4` follow (gateway-side)  
- [ ] Directory ingest of Riptide-shaped `PoolReport`  
