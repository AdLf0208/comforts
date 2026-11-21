# Comforts — Horizon port (0.2.0, SDK format)

A native Horizon port of **Comforts 9.0.5+1.21.1** (`illusivesoulworks/comforts`,
**LGPL-3.0-or-later**): a sleeping bag that sleeps through the night **without setting your
respawn point**, with upstream's server config (time window, wake offsets, `restrictSleeping`,
phantom rest, break chance).

**LOCAL-ONLY** until the maintainer approves publishing (ARCHITECTURE.md §8).
LGPL-3.0-or-later is licence rank 1 (copyleft): publishable under the same licence with the
complete corresponding source, which still needs the maintainer's approval.

This is the **flat SDK package** (mod API 0.2): Horizon's mod-API merge retired the `.hnmod`
container, so one `comforts.hmod` carries the code *and* the content. The retired 0.1.0 bundle
is kept as evidence at
`../_shared/evidence/comforts/retired-hnmod-0.1.0/`; the migration record is in
[`notes.md`](notes.md) §"Migration to the SDK format".

## Layout

```
horizon-mod.toml        the only manifest and identity (api 0.2, [native] pins, side both)
Cargo.toml              one crate; its only dependency is horizon-mod-api from .horizon/sdk/
src/lib.rs              the one #[mod_api::horizon_mod] impl Hooks + the glue and its tests
src/runtime.rs          the pure sleep rules (byte-identical to the retired runtime.rs, 15 tests)
assets/comforts/        models, blockstate, item icon, lang
data/comforts/          the block declaration, loot table, recipe, tags, creative tab
tools/e2e.sh            the in-game scenario (needs a launcher-built game; see the header)
comforts.hmod           the delivered package (built by `cargo horizon-mod build`)
FINDINGS.md             upstream analysis + the jar evidence
notes.md                the record: file map, hook table, ladder, deviations, gaps, skills
VALIDATION.md           the retired 0.1.0 verification table (historical)
```

`.horizon/sdk/` (the pinned SDK, fetched by any `cargo horizon-mod` command) and `target/` are
gitignored and never packed.

## What it does

- One block, `comforts:sleeping_bed` ("Sleeping Bag"), the vanilla white-bed template:
  `facing`/`part`/`occupied`, three pixels high, head-only self-drop, 3 white wool → 1.
- Right-click it to sleep. The engine's bed path does the rest (reach, obstruction, the monster
  check, the sleep list, the stat), so the night is skipped exactly as vanilla's is.
- **It never sets the respawn point.** `player_set_spawn` cancels the point when the block there
  is one of the port's beds; a vanilla bed in the same world still sets one, which is the
  in-game differential.
- `config/comforts-server.toml` (a world's `serverconfig/` copy wins), written from upstream's
  defaults by the game and re-read on change: `sleepingBagUse`, `daySleepingPercentage`
  (accepted, inert), `dayWakeTimeOffset`, `nightWakeTimeOffset`, `restrictSleeping`,
  `sleepingBagsStopPhantoms`, `sleepingBagBreakChance`.

## Not in this slice

The other 15 dyed bags, the 16 hammocks, `rope_and_nail`, the auto-use sleeping bag item, wake
effects, the luck multiplier, the SLP statistic and day sleeping as a block type. See `notes.md`
§"Gaps after the migration".

## Verify

```sh
ports/tools/build-slot.sh run --job comforts-sdk --port ports/comforts --step test -- \
    bash -c 'cd ports/comforts && CARGO_INCREMENTAL=0 cargo test --offline'          # 23 tests
ports/tools/build-slot.sh run --job comforts-sdk --port ports/comforts --step lint -- \
    bash -c 'cd ports/comforts && cargo fmt --check && cargo clippy --all-targets -- -D warnings'
rm -f Cargo.lock
ports/tools/build-slot.sh run --job comforts-sdk --port ports/comforts --step build -- \
    bash -c 'cd ports/comforts && cargo horizon-mod build'
cargo horizon-mod check comforts.hmod
cargo horizon-mod fits comforts.hmod --build nightly
```

The in-game leg needs a game the launcher built with this package
(`horizon-launcher native-build ports/comforts --instance <i> --trust-native-code`), which is
not installed in this environment: `tools/e2e.sh` takes `GAME=` (and an optional
`CONTROL_GAME=` without the mod) and keeps its `SELFTEST=1` fail-capable run.
