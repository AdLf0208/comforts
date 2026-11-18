# Comforts — port notes

> **Read [`## Migration to the SDK format`](#migration-to-the-sdk-format-020-2026-11-18) first:**
> it is the current state — version 0.2.0, one flat SDK `.hmod` at `0b14f8a`, the file map, the
> helper→hook table, the measured ladder and every trap. Everything above it is the history of
> the retired `.hnmod` bundle (pin `1d63e0a5`, `mixins/`, the staged ladder, the 0.1.0 in-game
> runs) and describes paths that no longer exist in this port.

Ported from **Comforts 9.0.5+1.21.1** (`illusivesoulworks/comforts`,
**LGPL-3.0-or-later**). Upstream analysis point: commit
`5ee1465755dcbfc0e47b5d53a1c61ad0b70cf178` (branch `1.21.1`, `gradle.properties`
`version=9.0.5+1.21.1`); [`FINDINGS.md`](FINDINGS.md) has the behaviour extracted
from it and the jar evidence.

**Class + tier:** content · **logic** (`ports/ledger.csv`), so the slice is
content (a bed-shaped block) plus behaviour hooks — no block entity. Approach
from `horizon/docs/modpacks.md` §"Tiers": *logic* = content + use/place/break
hooks + config; the tier is not re-tiered.

**Licence + rank:** **LGPL-3.0-or-later**, rank **1** (copyleft: publishable
under the same licence with the complete corresponding source). Read from the
jar's `META-INF/neoforge.mods.toml` (`license = "LGPL-3.0-or-later"`) and its
`LICENSE`/`COPYING.LESSER`. **Local-only** until the maintainer approves
publishing (ARCHITECTURE.md §8).

**Target:** Horizon **26.3** at the campaign pin
**`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`** (432 hooks / 174 helpers) with
horizon-mixins **`5a64af938ec86b00d53d16a408aabc0038fb3e57`** (schema 6), read
from the read-only checkouts `../_shared/pins/horizon-1d63e0a` and
`../_shared/pins/horizon-mixins-5a64af9`. Migration recipe:
`../_shared/repin-1d63e0a.md`. Slice 1 was implemented at `41f822b`; this is
**slice 2**, its migration to the pin and its delivery as one `.hnmod`.

**Status: slice 2 delivered and verified in game** (the ledger row moves to
`ported-partial` when the orchestrator records it). A sleeping bag that sleeps
through the night and **never sets the respawn point**, with upstream's config
keys, shipped as **one `.hnmod`** that carries and loads its own content (the
AE2 pattern). Day sleeping as a block type, hammocks, the 16 dyed bags, luck,
wake effects and the SLP statistic stay gaps (below).

## Why the earlier block was lifted

The 2026-11-05 probe ([`FINDINGS.md`](FINDINGS.md) §2) found the pin
`c9d474d`'s 108 hooks had no sleep/bed/time/spawn API, so the port was
`blocked(no-sleep-api)`. The campaign pin `41f822b` (schema 6) carries the
hooks that were added on 2026-11-02; the catalog is byte-identical at
`1d63e0a` (`repin-1d63e0a.md` §1), so the whole hook set still resolves at the
current pin. Each is re-verified here against `game.rs` of
`ports/_shared/pins/horizon-1d63e0a`:

| Hook | Dispatch at the pin | Used for |
|---|---|---|
| `can_player_sleep` (API 393) | `game.rs:15842` (`try_sleep`) | the bag's time window (`ComfortsEvents.checkTime`) and `restrictSleeping` |
| `player_bed_enter` (API 251) | `game.rs:15881` | remember the sleeper (start time, dimension, phantom-rest value) |
| `player_bed_leave` (API 252) | `game.rs:15946` (`wake_up`), `game/lan_actions.rs` (guest wake) | break chance on waking, phantom-rest restore |
| `player_deep_sleep` (API 256) | `game/lan_actions.rs:4709` | 100 ticks asleep: put the phantom-rest timer back |
| `player_spawn_point_set` (API 277) | `game.rs:15790` (`try_sleep`) | **cancel** the bag's bed spawn (the mod's whole point) |
| `sleep_finished_time` (API 308) | `game.rs:15991` (`tick_sleep`) | upstream's day/night wake offsets |
| `time_skip` (API 301) | `game.rs:15995` | **not used** — upstream has no veto, and cancelling would block `/time` |
| `register_configs` / `config_loading` / `config_reloading` / `config_unloading` (API 397) | `game/native_configs.rs` | `config/comforts-server.toml` |

Hook ids in the bundle: `embedded-content`, `register-configs`, `config-loading`,
`config-reloading`, `config-unloading`, `can-player-sleep`, `bed-enter`,
`bed-leave`, `deep-sleep`, `spawn-point`, `sleep-finished` (11 injections,
2 modules). `embedded-content` is the one **target** injection (a
`content_root` HEAD, as AE2 and Charging Gadgets use); the other ten name
stamped hooks.

### Two pin facts that shaped the port

1. **A bed is recognised by its id.** `tick_sleep` (`game.rs:15956`) and the
   guest sleep tick wake a sleeper whose block name does not end in `_bed`, and
   `respawn_anchor::is_bed` uses the same suffix. A bed-shaped content block
   therefore **must** end in `_bed`, so the block is `comforts:sleeping_bed`,
   not upstream's `comforts:sleeping_bag_<colour>` (the display name is still
   "Sleeping Bag"; recorded in `content/README.md`).
2. **Name comparisons make the vanilla no-spawn rule unreachable.**
   `bed_sets_spawn` and the free/destroy rules compare the *id* with
   `minecraft:straw_bed`, and a content block's facts are renamed to the mod
   id, so a mod bed sets the spawn unless `player_spawn_point_set` cancels it.
   That hook is the port's no-spawn rule; the e2e proves the vanilla-bed
   control still sets one.

## Implemented behaviour (slice 1, shipped by slice 2)

- **One block** `comforts:sleeping_bed` on the `minecraft:white_bed` template
  (bed class, `facing`/`part`/`occupied`), 3 px high (`shapes` and
  `collision_shapes`), hardness 0.1, wool sounds, `occludes: false`,
  head-only self-drop, 3 white wool → 1 recipe, `comforts:sleeping_bags`
  block/item tags, a creative tab and its item icon from upstream.
- **Sleeping** uses the engine's bed path (`Class::AbstractBed` →
  `PlayerEffect::Sleep` → `try_sleep`), so reach, obstruction, the monster
  check, the sleep list, the stat and the `slept_in_bed` trigger are vanilla's.
- **No spawn point**: `player_spawn_point_set` cancels the `bed` cause when
  the block at the position is a `comforts:*_bed` (upstream `canSetSpawn`).
- **The night is skipped** by the engine; `sleep_finished_time` applies
  upstream's offsets (`getWakeTime`). A day sleeper (`sleepingBagUse = DAY`)
  wakes at the coming dusk, 11 999 — also proven in the e2e.
- **Time window**: `can_player_sleep` maps `checkTime`'s ALLOW/DEFAULT/DENY
  onto the hook's `problem` (`None` / leave / `not_possible_now`).
- **Config** `config/comforts-server.toml` (a world's `serverconfig/` copy
  wins, as the pin's config dir does), written from upstream's defaults,
  re-read on change:
  | key | default | slice effect |
  |---|---|---|
  | `sleepingBagUse` | `NIGHT` | the time check (`NONE`/`DAY`/`NIGHT`/`DAY_OR_NIGHT`) |
  | `daySleepingPercentage` | `-1` | **accepted, inert** (day sleeping as a block type is deferred; the pin has no sleep-count override) |
  | `dayWakeTimeOffset` | `0` | wake time of a normal (night) sleep |
  | `nightWakeTimeOffset` | `0` | wake time of a day sleep (upstream swaps the two names; the port follows the code) |
  | `restrictSleeping` | `false` | refuses a sleep while `tired_until` has not passed |
  | `sleepingBagsStopPhantoms` | `true` | `false` puts the pre-sleep `time_since_rest` back |
  | `sleepingBagBreakChance` | `0` | after >500 ticks slept, breaks both halves with no drop |
- **`restMultiplier`** is fixed at upstream's default 2.0 (a slept-away night
  makes a player tired for half of it); the key is not registered (gap).

## Slice 2 — pin migration and the one-file delivery

- `compatibility.json` and the plan moved to `1d63e0a5` / `5a64af93`; the
  `game.rs` sha256 is the pin's (`64813cf4…f80f`), so staging fails loudly on
  the wrong checkout.
- **Embedded content (AE2 pattern):** `mixins/comforts/embedded_content.rs` +
  one `content_root` HEAD injection write `data/comforts-content.hmod` (the
  built `content/comforts.hmod`) to `.horizon-cache/` at start and append it
  to the dev-mod list. `tools/package.py` packs `mixins/<id>/data/` and
  refuses a copy that differs from the built `.hmod`; `tools/package.sh` does
  build → sync → package → `cargo horizon-mod check --kind native-mod`.
- **Content version normalised to the port's `0.1.0`** (the registry entry,
  the file and the release must agree); the plan's `[mod] version` is the same.
- `tools/horizon.py` gained the pin default and the `ctl` action (the port's
  own control client), and the embedded test filter is `game::comforts`, which
  covers both modules.
- Five support functions carry a narrow
  `#[expect(private_interfaces, reason = …)]`: the injected module is
  `pub(crate) mod comforts_state` under `crate::game` while the pin keeps
  `struct Game` private to that module, so the lint asks us to hide a function
  the helpers must call. They are our own glue, never a public API.
- The stale template tools (`tools/test_template.py`, `tools/test_stage.py`,
  `tools/probe.sh`) are the template's own fixtures from slice 1 and are not
  run for this port: `test_template.py` asserts the template's single
  `break_block` injection, which this bundle has not had since slice 1.

## Runtime TDD

`mixins/comforts/runtime.rs` is the pure engine and the standalone Cargo lib.
Slice 1 wrote its tests first and watched them fail (71 `E0425`/`E0433` errors)
before implementing. Slice 2 adds no rule, so the 15 runtime tests remain the
contract; support's 2 tests cover the config registration and the
loading/reloading/unloading path. Every number comes from the jar or upstream
source (`ComfortsConfig.java:68-152`, `ComfortsEvents.checkTime`/`getWakeTime`/
`onSleep`/`onWakeUp`, `SleepingBagBlock.canRest`).

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

## Verification

| step | command | result |
|---|---|---|
| runtime + glue unit tests (standalone) | `cargo test --locked --offline` (in a slot) | **15/15 passed** |
| fmt / clippy | `cargo fmt --all`, `cargo clippy --all-targets --locked --offline -- -D warnings` | clean, **0 warnings** |
| stage | `tools/horizon.py stage` | exit 0, pin `1d63e0a5`, stage 120–170 s |
| check (staged game) | `tools/horizon.py check` | `cargo_exit: 0`, **0 warnings** (`../_shared/evidence/comforts/check.log`) |
| test (embedded) | `tools/horizon.py test` | **17 passed; 0 failed; 1240 filtered** (`game::comforts::tests` 15 + `game::comforts_state::tests` 2; `../_shared/evidence/comforts/test.log`) |
| build (fast) | `tools/horizon.py build` | `cargo_exit: 0` |
| content build + check | `tools/package.sh` (pin's `cargo-horizon-mod`) | `comforts 0.1.0 (mod, API 0.2, both)`, **4,379 B**, `66fe01d9…80f4d`, 14 entries, "no code: content only" |
| `.hnmod` package + check | `tools/package.sh`, then `horizon-mod check … --kind native-mod` | **18,403 B**, `247d7937…6b210`, 16 entries, `native-1/1d63e0a5…/5a64af93…`, reproducible (two runs, same sha256) |
| in-game e2e, embedded | `build-slot.sh e2e -- tools/e2e.sh` | **PASS + EXIT:0** — `../_shared/evidence/comforts/e2e-embedded.log` |
| in-game e2e, selftest | `env SELFTEST=1 tools/e2e.sh` | **FAIL + EXIT:1** (the no-spawn assertion inverted) — `…/e2e-embedded-selftest.log` |

Delivered: `content/comforts.hmod` (4,379 B, `66fe01d9…`) and
`comforts.hnmod` (18,403 B, `247d7937…`, carrying
`mixins/comforts/data/comforts-content.hmod`). Both live in the port, not in
`target/`.

### What the e2e proves

- **EMBEDDED (default, no `--dev-mod`)**: the bundle writes
  `.horizon-cache/comforts-embedded-content.hmod`, byte-identical to
  `content/comforts.hmod`, and the game logs `content from 1 mod(s)`; the bag
  item and block exist.
- **CONTROL**: the same binary with `.horizon-cache` a plain file cannot write
  its content, and both ids are unknown (`Unknown item 'comforts:sleeping_bed'`,
  `Unknown block type 'comforts:sleeping_bed'`, the helper's warning in
  `latest.log`) — so the embedded leg is not vacuous.
- **A/B**: a night sleep in the bag skips the night (`day_time 18007 → 24009`,
  `time_of_day 10`), no "Respawn point set" line, no `respawn` key in the
  player NBT, and after a death the player respawns at the world spawn
  (`respawn_radius` set to 0, so it is exact) — not at the bag.
- **C** (differential): the vanilla white bed in the same world at the same
  hour shows "Respawn point set", writes `respawn` in the NBT and respawns the
  player at the bed. An implementation that ignored `player_spawn_point_set`
  fails B while C still passes.
- **D**: noon with `sleepingBagUse=NIGHT` is refused ("You can sleep only at
  night or during thunderstorms"); with `DAY` the same bag sleeps at noon and
  the day sleep skips to the dusk (`time_of_day 12008`, upstream's
  `getWakeTime`), then refuses at night — the config is live.
- **D2**: `restrictSleeping=true` refuses the next sleep ("You are not sleepy
  yet"); with it false the same click sleeps.
- **E**: `sleepingBagBreakChance=100` breaks both halves on waking with **no
  item dropped** (the floor count of bag items is unchanged), while the
  chance-0 bag from A survived.
- **F**: breaking the bag by hand returns one item (head-only loot table).

### Harness facts learned here (keep them)

- A single player's **Shift does not wake a sleeper** at this pin: the server's
  guest sleep tick only wakes a dead sleeper or one whose bed is gone, and the
  server `Game`'s own key state is empty. The e2e lets a sleep finish on its
  own (`finish_sleep`) instead of faking a wake-up.
- `respawn_radius` defaults to 10, and vanilla's `PlayerSpawnFinder` starts on
  a **random column** in that radius, so "respawned at the world spawn" is only
  an exact claim after `gamerule respawn_radius 0`.
- A survival death drops the inventory, so a later "the bag dropped nothing"
  check must count *bag items on the floor* before and after, not the presence
  of any bag entity; the e2e clears item entities first too.
- `ctl place` places against a face the player can see: teleport to the target
  minus two on one axis and aim at the top of the floor block under it.
- The arena is built around wherever the player spawned (a stone floor one
  block below the feet, six blocks of air), so the run never depends on the
  seed's terrain height. It is in the **Overworld**, not the End: a bed in the
  Nether or the End destroys itself on use (`beds_destroy_on_use`,
  `respawn_anchor.rs:287`), so sleeping cannot be observed there; the arena
  clears non-player entities and checks `screen == playing` before the input
  phases, which is what the End arena is for elsewhere.

## Deviations and gaps

- **Block id** `comforts:sleeping_bed` (upstream `comforts:sleeping_bag_white`)
  because of the pin's `_bed` suffix rule (above). Only the plain white variant
  is carried; the other 15 dyed bags, the 16 hammocks, `rope_and_nail`, the
  sleeping-bag **item** (auto-use on placement) and the block-entity model are
  deferred.
- **The bag's model is the port's**: upstream draws it from a Java
  `LayerDefinition` (a block-entity renderer, deep/client tier) which a data
  model cannot carry; the port draws a flat 3 px body and a pillow with vanilla
  wool textures and carries upstream's item icon.
- **`daySleepingPercentage` is inert** and **day sleeping is not enabled** by
  default (upstream's `DAY`/`DAY_OR_NIGHT` paths exist in the runtime and are
  triggered by config, but no hammock exists yet). The pin has no sleep-count
  override, so upstream's `MixinSleepStatus` share of the day percentage cannot
  be ported.
- **`sleepingBagsStopPhantoms=false`** is restored through `player_deep_sleep`
  (exact to a tick) and through `player_bed_leave` for a sleeper who wakes
  before 100 ticks (that path loses the ticks slept, under a thousand). The
  e2e does not exercise it: the phantom timer is not observable through
  `horizon-ctl` at this pin.
- **Wake effects, the luck multiplier, `sleepingBagBreakChanceLuckMultiplier`**
  and the SLP statistic/sleep-wake/tired-time capability are not ported; the
  tired time lives in memory for the session (`PLAYERS`), so it resets on a
  restart, unlike upstream's player capability.
- **The recipe is unconditional**: upstream gates it behind the
  `comforts:sleeping_bag_enabled` condition and the `enableSleepingBagRecipes`
  common config; 26.3 content has no recipe conditions, so there is no common
  config either.
- **Breaking a bed in survival leaves its other half standing** at this pin
  (the pin removes it only in creative). That is engine behaviour, not the
  port's; the e2e records it and finishes the foot off itself.
- Clients without the mod are refused (content `side = "both"`); the native
  bundle is not yet in the mod list, as for every port.
- The jar was re-fetched for this slice (the cached copy was gone): Modrinth
  project `SaCpeal4`, version `3kpPjcTc` (`9.0.5+1.21.1`), SHA-1
  `21d6bee3…087f` = the published record; kept at
  `carry-work/comforts-neoforge-9.0.5+1.21.1.jar` (this job may not write
  `ports/_shared/licence-cache/`).

## Files

```
ports/comforts/
├── Cargo.toml                     standalone lib (runtime.rs) + audit workspace
├── compatibility.json             pin 1d63e0a / mixins 5a64af9, game.rs hash
├── mixins/comforts/
│   ├── horizon-mixin.toml         2 modules + 11 injections
│   ├── runtime.rs                 pure sleep rules + 15 unit tests
│   ├── support.rs                 config, per-sleeper record, hook effects (+2 tests)
│   ├── embedded_content.rs        the content_root HEAD: writes and loads data/*.hmod
│   ├── data/comforts-content.hmod the built content mod inside the bundle
│   └── {register_configs,config_loading,config_reloading,config_unloading,
│        sleep_check,bed_enter,bed_leave,deep_sleep,spawn_point,sleep_finished}.rs
├── content/                       the .hmod sources + comforts.hmod (see its README)
├── comforts.hnmod                 the one deliverable (code + content)
├── carry-work/comforts-neoforge-9.0.5+1.21.1.jar   the jar this slice was read from
├── tools/{horizon.py,package.py,package.sh,e2e.sh}
└── notes.md / FINDINGS.md / VALIDATION.md / PLAN-slice2.md
```

## Skills and references used

- **superpowers** (loaded before acting): `using-superpowers`, `brainstorming`
  (re-scoped the slice from the jar inventory: the 16 bags/16 hammocks/rope set
  vs the one plain bag), `writing-plans` (`PLAN-slice2.md`),
  `test-driven-development` (the slice's rule set already had red→green tests;
  no rule changed), `systematic-debugging` (four e2e failures: the control
  world's terrain, the random spawn column, the sneak wake, the death drops —
  each traced to its cause before the fix), `verification-before-completion`
  (every claim below is a command and its output).
- **Beautiful Rust** (`rust-best-practices`, `rust-port`): expressive enums
  (`TimeUse`, `SleepCheck`), `Option` over sentinels, one private helper per
  hook, a narrow `#[expect(…, reason = …)]` instead of `allow`, `cargo fmt` /
  `clippy -D warnings` / `cargo test`.
- Campaign docs read: `AGENTS.md`, `ARCHITECTURE.md` §3–§10,
  `ports/comforts/{notes,FINDINGS,VALIDATION}.md`, the pin's `docs/modpacks.md`
  (tiers / carry-over / harness), `docs/mods.md` §content mods (API 0.2),
  `docs/native-mods.md` (the hook catalogue, "Data files", the one-`.hmod`
  ruling), `docs/datapack-upgrade.md`, `docs/horizon-ctl-commands.md`,
  `ports/_shared/AGENT_MOD_STRUCTURE.md`, `ports/_shared/build-slots-usage.md`,
  `ports/_shared/repin-1d63e0a.md`, `ports/_shared/remote-build.md`, the
  ledger row, the source-map row, and the reference ports `charginggadgets`
  (the embedded pattern it proved) and `restrictions`.

## Upstream source analysis (recorded per AGENTS.md)

| | |
|---|---|
| Repo | <https://github.com/illusivesoulworks/comforts> |
| Commit | `5ee1465755dcbfc0e47b5d53a1c61ad0b70cf178` (branch `1.21.1`, version 9.0.5+1.21.1) |
| Jar | `comforts-neoforge-9.0.5+1.21.1.jar`, 422,658 B, SHA-256 `6b0fd35a…e9615`, SHA-1 `21d6bee3…087f` (Modrinth `SaCpeal4`, version `3kpPjcTc`) |
| Files read | `common/.../ComfortsConfig.java`, `ComfortsEvents.java`, `block/BaseComfortsBlock.java`, `block/SleepingBagBlock.java`, `block/HammockBlock.java`, `item/SleepingBagItem.java`, `common/capability/*`; `forge/.../ForgeSleepEvents.java`; `common/src/main/resources/{assets,data}/comforts/**`; `neoforge/src/generated/resources/data/comforts/recipe/*`; the jar's `META-INF/neoforge.mods.toml`, `LICENSE`, `COPYING.LESSER` |
| Behaviours extracted | time window + two offsets (`checkTime`), wake time (`getWakeTime`), no-spawn (`canSetSpawn`), sleep flags (`restrictSleeping`, `sleepingBagsStopPhantoms`, break chance and its `>500` gate), config defaults and key names, the white-wool recipe and head-only loot |
| Where the rewrite differs | the block id (`_bed`), the model (data instead of a renderer), the tiered omissions above |
| Clone | `ports/_shared/upstream/comforts/` (gitignored) — analysis only, deleted after the findings |

The Java source is an analysis aid only: the port is a clean-room Rust rewrite
against the pin's hooks (no translated code, no decompiled jar).

## Review repairs (PR #664)

1. Staging is the orchestrator's (horizon-mods#686).
2. *"Correct the original-project URL … from relative `illusivesoulworks/comforts`"*
   — the entry links `https://github.com/illusivesoulworks/comforts`.
3. *"Resolve license evidence: entry/body say LGPL-3.0-or-later, but the linked
   repository root LICENSE contains MIT terms … Do not simply relabel derived
   LGPL code MIT."* — the jar's `META-INF/neoforge.mods.toml` says
   `license = "LGPL-3.0-or-later"` and its bundled `LICENSE` (Copyright (C)
   2017-2022 Illusive Soulworks) grants the LGPL; the jar also ships `COPYING`
   (GPL v3) and `COPYING.LESSER`. The repository `LICENSE` was the stale MIT
   text; it is now the **full GNU LGPL v3** (additional permissions followed by
   the GPL v3) with Illusive Soulworks' notice, and `LICENSE-NOTICE.md` records
   the evidence. Entry and files agree with upstream. The archive carries both.

### Verification after the repairs

- Artifact: `comforts.hnmod`, 48,941 B, SHA-256 `9a210bc9d6d1c5f7e2d10486bb15aca6c78e4b156e0c0b5bd0f952edd91c2c06` (rebuilt twice, identical bytes; `tools/package.py`; the embedded content equals `content/comforts.hmod`).
- `upstream_ready.py comforts --copyleft` exits 0 (READY, LGPL-3.0-or-later). `check-skills.py --notes ports/comforts` exits 0.

## Migration to the SDK format (0.2.0, 2026-11-18)

Horizon's mod-API merge (#16941, launcher ≥ 0.2.70, PR #17992) retired the `.hnmod`
container, and the registry closed every `.hnmod` submission on 2026-11-18 with one reason.
This port's former submission, **PR #797** (`comforts` 0.1.0), was closed by that purge;
**0.2.0** is the same behaviour republished as one flat SDK `.hmod`. The changelog text for the
resubmission is at the end of this section.

Pins: SDK nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (2026-11-18,
`package_format` 6, `content_api` "0.2"), mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, `cargo-horizon-mod` from
`horizon-rust/horizon@main`. **No `horizon-launcher` is installed here**, so no game exists at
`0b14f8a`: the in-game leg is a recorded gap, not a claim (see *Gaps after the migration*).

**Order followed:** the retired paths were **moved** to
`../_shared/evidence/comforts/retired-hnmod-0.1.0/` *before* the first `cargo horizon-mod`
command, `src/runtime.rs` was copied with `cp` and proved with `cmp` + `sha256sum`, `cargo test`
ran on the new crate with **only** `src/runtime.rs` in it (15/15 green), and only then was the
glue written — first as `todo!()` stubs with its 8 tests (red: `not yet implemented`), then
implemented (23/23 green).

### File map

| 0.1.0 path | Now | Evidence |
|---|---|---|
| `mixins/comforts/runtime.rs` | `src/runtime.rs`, **byte-identical** (sha256 `9959046c…0bc5`) | `retired-hnmod-0.1.0/mixins/comforts/runtime.rs` |
| `mixins/comforts/support.rs` + 11 hook helpers | `src/lib.rs` (the one `Hooks`, the `Mod` state, the glue and 8 tests) | `retired-hnmod-0.1.0/mixins/comforts/` |
| `mixins/comforts/horizon-mixin.toml` (2 `[[module]]`, 11 `[[injection]]`) | no plan: typed `Hooks` methods; nothing is a Part | `retired-hnmod-0.1.0/mixins/` |
| `mixins/comforts/data/comforts-content.hmod` + `embedded_content.rs` (the AE2 write-out) | **gone**: `assets/` and `data/` are inside the one package | `retired-hnmod-0.1.0/mixins/` |
| `content/` (`horizon-mod.toml`, `assets/`, `data/`, `comforts.hmod`, `README.md`) | the port root: `assets/`, `data/`; the content manifest merged into the root `horizon-mod.toml` | `retired-hnmod-0.1.0/content/` |
| `comforts.hnmod` (48,941 B, carried the content) | `comforts.hmod` (39,570 B, code + content) | `retired-hnmod-0.1.0/comforts.hnmod` |
| `compatibility.json`, `rust-toolchain.toml` | `[native]` in `horizon-mod.toml`; the pinned commit decides the toolchain | `retired-hnmod-0.1.0/` |
| `tools/horizon.py`, `package.py`, `package.sh`, `test_stage.py`, `test_template.py`, `add-notices.py`, `tools/audit/`, `.github/workflows/ci.yml`, `probe.sh` | deleted: staging, `.hnmod` packaging, the plugin audit and the template's fixtures belong to the retired format | `retired-hnmod-0.1.0/tools/`, `…/.github/`, `…/probe.sh` |
| `carry-work/comforts-neoforge-9.0.5+1.21.1.jar` | the jar is evidence, not a package path: moved to `../_shared/evidence/comforts/carry-work/` | that folder |
| `PLAN-slice2.md`, the old repo `AGENTS.md` | historical (both told the reader to open `compatibility.json` and run `tools/horizon.py`) | `retired-hnmod-0.1.0/`; a new `AGENTS.md` describes the SDK package |
| `Cargo.lock` | not kept: it is packed and changes the artifact (trap 3) | not kept |

**Kept and updated:** `tools/e2e.sh` (same scenario and assertions; it now takes `GAME=` for a
launcher-built game, an optional `CONTROL_GAME=`, and keeps `SELFTEST=1`), `notes.md`,
`README.md` (rewritten for the flat package **before** the first build), `FINDINGS.md`,
`VALIDATION.md` (marked historical), `LICENSE`, `LICENSE-NOTICE.md`.

**Carried content is unchanged:** all 13 files of `content/{assets,data}` match their package-root
copies byte for byte (`../_shared/evidence/comforts/sdk-carried-files.txt`: 13 × `unchanged`).

### Manifest mapping

- identity: `id = "comforts"` (also the resource namespace), `name = "Comforts"`,
  `version = "0.2.0"` (the port's version — a resubmission is always a new version),
  `authors = ["Illusive Soulworks", "AdLf0208"]`, `description` naming the slice boundary.
- `api = "0.2"` (exact), `side = "both"` (an integrated server belongs to the client
  installation, so single player and a LAN host need it), `capabilities = ["registries"]`
  (the schema's one value; this package really does declare a block).
- `[native] game = "26.3"`, `horizon = "0b14f8a…"`, `mixins = "e59da90…"` replace
  `[compatibility] horizon_revision` and the bundle's `5a64af93` mixins revision.
- `[mod] priority = 100` is gone; nothing in this slice needs ordering.
- `license`, `homepage` and `issues` are **not** manifest fields at this schema; the licence
  travels as `Cargo.toml`'s `license` plus the two licence files inside the package.

### Helper → hook table

Every retired injection has a home; **no hook is a gap**.

| Retired injection (pin `1d63e0a5`) | SDK 0.2 | Mapping notes |
|---|---|---|
| `[[module]] runtime` | `src/runtime.rs` as an ordinary module (`pub mod runtime;`) | byte-identical |
| `[[module]] state` (`support.rs`, `static CONFIG`/`PLAYERS`) | `Mod { config: Mutex<Option<Config>>, players: Mutex<BTreeMap<u128, PlayerSleep>> }` | `Hooks` is `Send + Sync`; a `Mutex`, never a `RefCell` |
| `embedded-content` (`content_root` HEAD, wrote `data/*.hmod`) | **none** — the flat package ships `assets/` + `data/` itself | — |
| `register-configs` (`NativeConfig::new(Server, DEFAULT_CONFIG)`) | `Hooks::register_configs` → `ModConfig::new("comforts", ConfigKind::Server, DEFAULT_CONFIG)` | same `config/comforts-server.toml`, same defaults text |
| `config-loading`, `config-reloading` (`NativeConfigFile`) | `Hooks::config_loading`, `Hooks::config_reloading` → `read_config` | `ConfigFile { mod_id, kind, text }`; another mod's file is ignored |
| `config-unloading` | `Hooks::config_unloading` → `drop_config` | defaults again, sleepers cleared |
| `can-player-sleep` (API 393) | `Hooks::can_player_sleep` → `problem_after` | writes `event.problem`; `NotSleepy` also sends the action-bar line |
| `bed-enter` (API 251) | `Hooks::player_bed_enter` (returns `Flow`) | always `Continue`; records dimension/start/whether the sleep skips the day |
| `bed-leave` (API 252) | `Hooks::player_bed_leave` → `take_sleeper`, `should_break`, `break_bag` | break roll from the level's own random, both halves set to air, wool break sound |
| `deep-sleep` (API 256) | `Hooks::player_deep_sleep` → `deep_delta` | `player.stat` / `player.award_stat` on `minecraft:time_since_rest` |
| `spawn-point` (`player_spawn_point_set`, API 277) | `Hooks::player_set_spawn` (returns `Flow`) | the retired hook named `cause = "bed"`; this one names `pos: Option<[i32;3]>`, so the block there answers it |
| `sleep-finished` (`sleep_finished_time` v1, API 308) | `Hooks::sleep_finished_time` **v2** → `wake_target` | writes `ClockAdjustment::Absolute(target)`, not `new_time` |

### Behaviour changes caused by the mapping (deliberate)

1. **The action-bar line goes through the player handle.** `tell_sleeper` used to push a guest
   packet by hand for a LAN guest and set `self.message` for the host; `PlayerRef::action_bar`
   with the same translatable key (`capability.comforts.not_sleepy`,
   `block.comforts.sleeping_bag.broke`, both in the carried `en_us.json`) is the SDK path and
   covers host and guests alike.
2. **The break roll draws from the level's stream and stores it back.** `level.random(host)` is
   a *copy*; drawing without `set_random` would leave vanilla's later draws unmoved. A level
   whose stream cannot be read does **not** break the bag (`should_break(…, None)` is false)
   instead of treating a missing roll as `0.0`, which would break at every non-zero chance.
3. **Block writes are flags `NEIGHBORS | CLIENTS`** (`3`, the retired `REMOVE_FLAGS`) through
   `LevelRef::set_block`, with `minecraft:air` resolved by `host.block_state_id`.
4. **The foot is found from the state's `facing`, not an internal helper.** `level.block_property(pos,
   "facing")` feeds `foot_pos`, a port copy of the pin's own `bed_forward` table and the only
   place the mapping table lives; it has a unit test.
5. **The spawn rule no longer keys on a `cause`.** It cancels when the block at the new point is
   one of the port's beds. That also cancels a `/spawnpoint` aimed at a bag, which is what
   upstream's `canSetSpawn` does (it tests the block), so this is the faithful reading rather
   than a widening.
6. **The wake time is an adjustment.** Contract v2 hands `new_time` read-only and
   `adjustment: &mut ClockAdjustment`; `Absolute(target)` from `runtime::wake_time` is the same
   clock move the v1 write made. `current` is now the level's own clock time.
7. **`day_time` is fallible.** `level.day_time(host) -> Option<i64>`; `None` leaves the decision
   to vanilla rather than running the rules at 0.
8. **Stats are the player handle's.** `game.stats.get/add("custom", …)` became
   `player.stat` / `player.award_stat(StatType::Custom, "minecraft:time_since_rest")`.

### Build and verify (measured 2026-11-18)

Every command ran inside a `ports/tools/build-slot.sh` lease with `BUILD_LOCAL=1` (the job id
`comforts-sdk`, steps `test`, `lint`, `fmt`, `build`, `check`), **directly in the port** — there is
no staging step and no staged tree in the SDK format. `CARGO_INCREMENTAL=0`, no `--target-dir`.

| Step | Command | Result |
|---|---|---|
| runtime only, before glue | `cargo test --offline` | **15 passed**, 0 failed (`evidence/comforts/sdk-test-runtime-only.log`) |
| glue red | `cargo test --offline` with `todo!()` stubs | **15 passed; 8 failed**, `not yet implemented` (`sdk-test-red-glue.log`) |
| runtime + glue | `cargo test --offline` | **23 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, **0 warnings** (`sdk-clippy.log`) |
| format | `cargo fmt --check` (**not** `--all`) | clean (`sdk-fmt.log`; the pre-format diff is `sdk-fmt-before.log`) |
| package | `rm -f Cargo.lock && cargo horizon-mod build` | `Built ./target/horizon-mod/comforts.hmod` (`sdk-build.log`) |
| reproducible | second `rm -f Cargo.lock && cargo horizon-mod build` | **same bytes** (`cmp` clean), same 39,570 B, same sha256 (`sdk-build-second.log`) |
| registry check | `cargo horizon-mod check comforts.hmod` | exit 0, output below (`sdk-check.log`) |
| fits the build | `cargo horizon-mod fits comforts.hmod --build nightly` | exit 0, output below |
| `runtime.rs` untouched | `cmp` + `unzip -p comforts.hmod src/runtime.rs \| sha256sum` | `9959046ca5a8eb3ac67aa52b6f2841602cf8344c7017fd5e9f161f9d18009bc5`, both (`sdk-runtime-carry.log`) |
| uses | `unzip -p comforts.hmod uses.json` | 169 entries (`sdk-uses.json`, `sdk-uses-summary.txt`) |
| skills line | `python3 ports/tools/check-skills.py --notes ports/comforts` | `Skills line present`, **SKILLS GATE: PASS** |

`check` (verbatim):

```
comforts 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  39570 bytes, SHA-256 b9f3b8963a47e6069a4a94da87b905d396bf278a17445296a8a6b465924d38ef, unsigned
  native source: unsandboxed; not executed by the registry check. Compile and test against the exact pinned Horizon and Mixins revisions before review.
```

`fits` (verbatim):

```
Comforts:
0b14f8a  made for
```

### Package

`comforts.hmod` — **39,570 B**, SHA-256
`b9f3b8963a47e6069a4a94da87b905d396bf278a17445296a8a6b465924d38ef`, **21 entries**, built twice
with identical bytes. Entries: `horizon-mod.toml`, `Cargo.toml`, `LICENSE`, `LICENSE-NOTICE.md`,
`README.md`; `src/lib.rs`, `src/runtime.rs`; 7 `assets/comforts/**` (blockstate, item, lang, two
models, item model, texture); 6 `data/comforts/**` (block declaration, creative tab, loot table,
recipe, two tags); `uses.json`. **No `Cargo.lock`** (`sdk-entries.log`).

`uses.json` is 169 entries: 11 hook names (our ten plus an over-approximated `hook:play_sound`),
19 hook fields, 17 methods, 28 handle methods, 79 types, 12 traits, `capability:registries`,
`macro:horizon_mod` and `part:block`. The checker over-approximates from names in the source (it
also lists `method:Hooks::play_sound` and `ServicesServerHost::players`, which this mod never
calls as hooks), so `sdk-uses-summary.txt` is the honest reading; `check` passes with them.

Evidence: [`ports/_shared/evidence/comforts/`](../_shared/evidence/comforts/) — the `sdk-*.log`
files above, `sdk-uses.json`, `sdk-uses-summary.txt`, `sdk-sha256.txt`, `sdk-entries.log`,
`sdk-carried-files.txt`, `sdk-runtime-carry.log`, and the retired tree under
`retired-hnmod-0.1.0/`.

### Traps this migration hit (new to this port)

1. **`ConfigFile` is `#[non_exhaustive]`: a test cannot build one.** `ConfigFile { … }` is
   `E0639`. Split the matching rule out of the hook (`read_config_text`/`drop_config_of` taking
   `(mod_id, kind, text)`) instead of reaching for a fake file.
2. **`sleep_finished_time` v2 does not accept a written time.** `*event.new_time = …` does not
   compile (`i64` is read-only); the field you write is `*event.adjustment =
   ClockAdjustment::Absolute(…)`, and it is a `&mut` behind the event (one deref, not two).
3. **`player_set_spawn` has no `cause`.** The retired hook's `cause == "bed"` test has no
   counterpart; `pos` is `Option<[i32;3]>` (None for a player with no point), so the rule is the
   block at `pos`. Any port that copies `cause` from the `.hnmod` glue will not compile.
4. **`level.random()` is a copy.** Draw from it and `set_random` it back, or vanilla's later
   draws do not move; a `None` stream must not be read as a `0.0` roll (it would break the bag
   at any non-zero chance).
5. **`host.level(dimension)` is on `EntitiesHost` and `host.dimension()` on `ConfigHost`.**
   Neither is where "levels" suggests; both arrive through `Host`.
6. **A `BlockKey::id(host)` is a `String`.** Perfectly fine per check, but do not hold one across
   a write — the checks here re-read the block each time.
7. **`carry-work/` is not a package path either.** The jar had to move to evidence with the rest;
   keeping it in the port would fail `cargo horizon-mod build` the same way `mixins/` did.
8. **Stale root documents get packed.** `probe.sh`, `PLAN-slice2.md` and the old `AGENTS.md` were
   the template's/slice's leftovers; every root document that is not a package file still ends up
   in the zip, so they moved out and `README.md` was rewritten **before** the first build.
9. **The content manifest must not be copied to the root.** `content/horizon-mod.toml` declares
   `id`/`version` for the content half; the root manifest is the only identity, so its fields
   were merged (and `[native]` added) instead of both files shipping.
10. **`cargo horizon-mod build` really does pack a `Cargo.lock`.** The first exploratory build
    (before `rm -f Cargo.lock`) produced 22 entries with a 8,829 B lock and a different hash;
    every recorded build starts with `rm -f Cargo.lock`. `/.gitignore` carries `/Cargo.lock`.
11. **The 0.1.0 e2e cannot run the new package.** It launched a game built by `tools/horizon.py`
    and loaded the bundle's embedded content; the SDK package needs
    `horizon-launcher native-build`, which is not installed here. The script now takes `GAME=`
    and an optional `CONTROL_GAME=`, and keeps `SELFTEST=1`.
12. **`can_player_sleep` has three answers, not two.** Returning "keep vanilla's problem" is
    right only for `SleepCheck::Default`; an **allowed** sleep must set `problem = None`, or a
    `DAY` bag cannot sleep through vanilla's own refusal. A direct port of the 0.1.0 `match`
    loses that if the enum is collapsed to keep/refuse.

### One mapping bug found by self-review before delivery

The first glue draft modelled the hook's answer as *keep / refuse-now / not-sleepy* and left a
vanilla problem in place when the config **allowed** the sleep. Upstream's `checkTime` clears it
(`problem = None`), which is exactly what makes a `sleepingBagUse = DAY` bag work at noon with
vanilla's "not possible now" in the way. The port now has `Problem::Clear` (`*event.problem =
None`), with the two assertions that would have caught it (an allowed day sleep clears vanilla's
problem, the monster check included; `restrictSleeping` still applies after the window allows).
Written test-first: the assertions failed to compile (`no variant Clear`) before the fix.

### Gaps after the migration

Everything in *Deviations and gaps* above still stands (the block id, the port model,
`daySleepingPercentage` inert, the session-only tired timer, wake effects/luck/SLP, the
unconditional recipe, the pin's half-breaking bed). Added by this format:

1. **No in-game run at `0b14f8a`.** No `horizon-launcher` and no game binary at this build, so
   no `ctl` transcript exists for 0.2.0. What is proven here: every retired hook maps to a typed
   method at the same vanilla call site, the glue type-checks against the pinned SDK, the 23
   tests pass, the package passes `check`/`fits`, and every rule is byte-identical. The 0.1.0
   runs (`e2e-embedded.log`, `e2e-embedded-selftest.log`, `check-e2e-logs.py comforts --strict`
   → 2/2) remain the behaviour's evidence for a `runtime.rs` that did not change; they are marked
   historical, not re-claimed.
2. **No logger.** The SDK exports none and a native mod may not add a crate, so the retired
   `log::warn!` on a failed block write is dropped (the write's `Result` is ignored). The e2e
   already asserts the *effect*, not the log line.
3. **The tired timer is session state.** Unchanged from 0.1.0: it is a `Mutex<BTreeMap<…>>` field
   and resets on restart; upstream keeps it in a player capability.
4. **`player.award_stat` with a negative delta** is how the phantom timer is put back (the retired
   `stats.add` did the same). The SDK documents the additive call, so a negative amount is the
   same arithmetic but is unproven in game here (recorded, not faked).

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port

### Changelog for the resubmission (0.2.0)

> **0.2.0 — republished as an SDK `.hmod`.** Horizon's mod-API merge (launcher ≥ 0.2.70)
> retired `.hnmod` and closed **#797** (`comforts` 0.1.0); this version supersedes it. One flat
> `comforts.hmod` now carries the code *and* the content (block, models, item icon, recipe, loot,
> tags, lang, creative tab), so the embedded-content injection and the AE2 cache write are gone.
> The ten stamped hooks are typed `Hooks` methods: `can_player_sleep`, `player_bed_enter`,
> `player_bed_leave`, `player_deep_sleep`, `player_set_spawn` (the retired
> `player_spawn_point_set`, which no longer carries a `cause`, so the block at the point answers
> the no-spawn rule) and `sleep_finished_time` at contract **v2**, which moves the clock through
> `ClockAdjustment::Absolute` instead of writing `new_time`; the config is registered and
> reloaded through the four config hooks, and the break roll draws from the level's own random
> stream and stores it back. `runtime.rs` and its 15 tests are byte-identical to 0.1.0, so the
> time window, the wake offsets, `restrictSleeping`, the phantom restore and the break chance
> behave exactly as before. Built against SDK `0b14f8a` / mixins `e59da90` for 26.3; the in-game
> leg still needs a launcher-built game (no `horizon-launcher` here) and is the one recorded gap.
