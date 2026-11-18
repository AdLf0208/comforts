# Comforts — a Horizon port

**Comforts** is a Minecraft mod. Minecraft Mod: Adds sleeping bags and hammocks for, respectively, portability and turning day to night, without setting new spawns. Comes in 16 different colors!. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [illusivesoulworks/comforts](https://github.com/illusivesoulworks/comforts) — licence **LGPL-3.0-or-later**.

| | |
|---|---|
| Upstream mod | Comforts ([source](https://github.com/illusivesoulworks/comforts)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `comforts.hmod` (install it with the Horizon launcher) |
| Licence | **LGPL-3.0-or-later** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

### Not in this slice

The other 15 dyed bags, the 16 hammocks, `rope_and_nail`, the auto-use sleeping bag item, wake
effects, the luck multiplier, the SLP statistic and day sleeping as a block type. See `notes.md`
§"Gaps after the migration".

## Not yet ported

- **Block id** `comforts:sleeping_bed` (upstream `comforts:sleeping_bag_white`)
- **The bag's model is the port's**: upstream draws it from a Java
- **`daySleepingPercentage` is inert** and **day sleeping is not enabled** by
- **`sleepingBagsStopPhantoms=false`** is restored through `player_deep_sleep`
- **Wake effects, the luck multiplier, `sleepingBagBreakChanceLuckMultiplier`**
- **The recipe is unconditional**: upstream gates it behind the
- **Breaking a bed in survival leaves its other half standing** at this pin
- Clients without the mod are refused (content `side = "both"`); the native

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `comforts.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Comforts was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **LGPL-3.0-or-later**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/illusivesoulworks/comforts

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run comforts` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/comforts/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
