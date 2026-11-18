# Comforts — upstream findings and pin probe

> The upstream analysis and the jar evidence below are current. The **pin** probe (§2–§6) is
> the 2026-11-05 `c9d474d` record that first blocked the port and then unblocked it at
> `41f822b`/`1d63e0a5`; both belong to the retired `.hnmod` format. The SDK package
> (`0b14f8a`, mod API 0.2) and its hook mapping are in [`notes.md`](notes.md)
> §"Migration to the SDK format".

**Port:** `comforts` (Comforts 9.0.5+1.21.1, LGPL-3.0-or-later)
**Upstream:** <https://github.com/illusivesoulworks/comforts>
**Commit analysed:** `5ee1465755dcbfc0e47b5d53a1c61ad0b70cf178` (branch `1.21.1`,
`version=9.0.5+1.21.1`; the 26.x branch is `16.0.0+26.2`, same sleep design)
**Pin probed:** Horizon `c9d474dd9116e84c243f8992d75c11314df56f06`
(tag `nightly-source/2026-11-01-c9d474d`), mixins `d9c83d7`
**Minecraft assets:** `26.3.zip` (`manifest.json` → `minecraft.version=26.3`);
prepared root `/home/adlf/Desktop/horizon/.local/horizon-client/26.3` exists.
**Decision at that pin: BLOCKED** — the pin exposes no sleep-start, time-skip,
spawn-point or player-pose API (§2–§6 are that probe's evidence). **Resolved
2026-11-07/11-11**: the campaign pin `41f822b`/`1d63e0a5` carries those hooks;
slice 1 was implemented and slice 2 migrated it to `1d63e0a5` and shipped it as
one `.hnmod` (see `notes.md` and `VALIDATION.md`).

## 0. Jar evidence (slice 2, read from the artefact)

The cached jar was gone; this slice re-fetched the exact pack version from
Modrinth (project `SaCpeal4`, version id `3kpPjcTc`, `version_number`
`9.0.5+1.21.1`, published 2026-01-24) and kept it at
`carry-work/comforts-neoforge-9.0.5+1.21.1.jar`:

| | |
|---|---|
| File | `comforts-neoforge-9.0.5+1.21.1.jar`, 422,658 B |
| SHA-256 | `6b0fd35a1349107e08a45539adbde9683bb203febc43a3305f6fc4ac73e59615` |
| SHA-1 | `21d6bee3821d62fe3356c51fb1d697871a3087ff` (equals Modrinth's published hash) |
| Licence | `license = "LGPL-3.0-or-later"` in `META-INF/neoforge.mods.toml`; `LICENSE` = LGPL-3.0-or-later (Illusive Soulworks 2017-2022), `COPYING`/`COPYING.LESSER` = the LGPL text; it also redistributes SpectreLib under LGPL-2.1 |
| Mod id / version | `comforts` / `9.0.5+1.21.1` |

`unzip -l` inventory (what the port could carry): **16 sleeping bags**
(`assets/comforts/blockstates/sleeping_bag_<colour>.json`, each a single
`white_cloth`-style model — the bag is drawn by a block entity, so the
blockstate has no variants), **16 hammocks**, `rope_and_nail`, 16 cloth
textures per family, 15 lang files, `data/comforts/loot_table/blocks/
sleeping_bag_<colour>.json` (head-only, `survives_explosion`, copies
`custom_name`), `data/comforts/recipe/sleeping_bag_<colour>.json` (3 wool in a
column, gated by the `comforts:sleeping_bag_enabled` condition), the dye
recipes, `data/c/tags/item/ropes.json`, `assets/minecraft/atlases/blocks.json`,
`assets/comforts/seamless_rules/*`, and the Java classes under
`com/illusivesoulworks/comforts/**` (not read: the port is a clean-room rewrite
from behaviour and data files).

Slice 1 carries the **plain sleeping bag** as `comforts:sleeping_bed` (the
`_bed` suffix the pin requires) and records the other 15 colours, the hammocks
and `rope_and_nail` as gaps; see `notes.md` §"Deviations and gaps".

---

## 1. What the mod does (upstream, file:line)

Comforts adds two two-part `BedBlock` subclasses, 16 dyed colours each:

- `BaseComfortsBlock extends BedBlock` with `part` (foot/head), `occupied` and
  `waterlogged` (`common/block/BaseComfortsBlock.java:66-78`).
- Right-clicking the head calls `trySleep` (`:86-149`). `trySleep` is a
  reimplementation of `ServerPlayer.startSleepInBed` that relaxes the time
  check per block type (`:151-216`).
- The success path is **`player.startSleeping(at)`**, sleep counter 0, the
  `SLEEP_IN_BED` stat, the `SLEPT_IN_BED` criterion and
  `ServerLevel.updateSleepingPlayerList()` (`:200-214`). That is the engine
  sleep-start + sleep-list update; Comforts does not implement it itself.
- **No spawn point is set.** `ComfortsEvents.canSetSpawn` returns `false` for
  both block classes (`common/ComfortsEvents.java:50-59`); unlike a vanilla
  bed, the use path never calls `setRespawnPosition`. The mod's whole point is
  "portable sleep without changing your spawn".
- Time-of-day rules: `sleepingBagUse=NIGHT`, `hammockUse=DAY`,
  `daySleepingPercentage=-1` (use the game rule), `dayWakeTimeOffset=0`,
  `nightWakeTimeOffset=0`, `restrictSleeping=false`, `restMultiplier=2.0`,
  `sleepingBagsStopPhantoms=true`, `hammocksStopPhantoms=true`,
  `sleepingBagBreakChance=0`, luck multiplier 0, effects empty
  (`common/ComfortsConfig.java:68-152`).
- The day span is `[100, 11900]` with the two offsets
  (`ComfortsEvents.checkTime`, `:61-87`). A day sleeper moves the wake time to
  the next night (`getWakeTime`, `:89-123`); `MixinSleepStatus` lowers the
  number of sleepers needed during the day (`mixin/MixinSleepStatus.java:23-37`)
  and `MixinServerSleepStatus`/`announceSleepStatus` shows
  `comforts.skipping_day` (`:262-294`).
- Sleeping bag auto-use: placing with `autoUse=true` immediately tries to
  sleep one block above (`common/item/SleepingBagItem.java:30-66`); on wake
  after >500 ticks the bag can apply configured effects and may break
  (`ComfortsEvents.onWakeUp`, `:128-204`). `canRest` controls the phantom
  timer reset (`SleepingBagBlock.java:62-65`, `HammockBlock.java:74-77`).
- Hammock placement: click a horizontal face and place into the adjacent
  replaceable block (`common/block/HammockBlock.java:117-129`); the
  `rope_and_nail` support is released when either half is removed
  (`:79-87`, `:104-114`).
- Per-player sleep data (sleep/wake/tired time, auto-sleep pos) is a capability
  (`common/capability/ISleepData.java:25-54`, `SleepDataImpl.java:23-91`).

Everything that matters — start sleeping, update the sleep list, skip time,
veto the spawn point — is engine behaviour that the mod reaches through
`startSleeping`/`updateSleepingPlayerList`/the `SleepStatus` and `ServerLevel`
mixins. A Horizon port must get those from the pin's mod API.

## 2. Probe: the pin's four needs

The hook catalogue at the pin is `native-hooks.json` (108 hooks). Reproduce
with [`probe.sh`](probe.sh).

| Need | Pin API | Verdict |
|---|---|---|
| Sleep start at a mod block (observe the attempt, put the player in bed) | No sleep/bed hook in the 108-hook catalogue. The only engine path is `Class::AbstractBed` → `PlayerEffect::Sleep` (`crates/horizon-world/src/level/interact.rs:344-365`) → private `Game::try_sleep` (`game.rs:13571`). | **missing** |
| Time skip / wake time | Read-only `native_day_time` (`native_hooks.rs:3253`) and `native_clock` (`:3747`). No setter. `native_run_command` can run `/time set day` (`native_server.rs:480`) but that is a console clock command, not a sleep skip, and it returns `false` inside a player hook. | **missing** |
| Spawn point (veto it) | No native read/write/veto. `Game.respawn_config` is a pin-internal field (`game.rs:2509`) written by `try_sleep`; `bed_sets_spawn` only exempts the literal `minecraft:straw_bed` (`respawn_anchor.rs:588-590`). | **missing** |
| Player pose / sleep state | `NativeServerPlayer` is read-only `position`/`yaw`/`pitch`/`health` (`native_server.rs:19-37`). No player write API; `native_teleport_mob` is mob-only (`native_mobs.rs:138`); `set_sleeping` is the pin-internal guest sync (`lan_actions.rs:3854`). | **missing** |
| Sleep-count opt-out | `native_sleep_ignored(uuid, ignored)` (`native_server.rs:385`) | exists, but only removes a player from `players_sleeping_percentage`; it does not start sleep. |

`mob_spawned` is mob spawning, not a player spawn point; it is unrelated.

## 3. Probe output (pin c9d474d)

`./ports/comforts/probe.sh` (exit 0):

```
campaign: /home/adlf/Desktop/all_the_mods_10_rust_rewrite
horizon:  /home/adlf/Desktop/all_the_mods_10_rust_rewrite/horizon
pin:      c9d474dd9116e84c243f8992d75c11314df56f06 (c9d474d)

== 1. pin hook catalog ==
hooks: 108
sleep/bed/time/spawn/pose hooks: NONE

== 2. newer sleep/bed/time/spawn APIs at the pin (expect none) ==
ABSENT:  player_bed_enter
ABSENT:  player_bed_leave
ABSENT:  sleep_finished_time
ABSENT:  time_skip
ABSENT:  player_spawn_point_set

== 3. the same symbols exist after the pin (HEAD) ==
player_bed_enter         HEAD:crates/horizon-client/src/game/native_hooks.rs:3507:    pub(super) fn native_player_bed_enter(
sleep_finished_time      HEAD:crates/horizon-client/src/game/native_hooks.rs:4567:    pub(super) fn native_sleep_finished_time(
time_skip                HEAD:crates/horizon-client/src/game/native_hooks.rs:4578:    pub(super) fn native_time_skip(
player_spawn_point_set   HEAD:crates/horizon-client/src/game/native_hooks.rs:3632:    pub(super) fn native_player_spawn_point_set(

== 4. what the pin does expose (read-only) ==
c9d474dd9116e84c243f8992d75c11314df56f06:crates/horizon-client/src/game/native_hooks.rs:3244:    pub(super) fn native_weather(&self) -> (f32, f32) {
c9d474dd9116e84c243f8992d75c11314df56f06:crates/horizon-client/src/game/native_hooks.rs:3253:    pub(super) fn native_day_time(&self) -> i64 {
c9d474dd9116e84c243f8992d75c11314df56f06:crates/horizon-client/src/game/native_hooks.rs:3747:    pub(super) fn native_clock(&self) -> Option<NativeClock> {
c9d474dd9116e84c243f8992d75c11314df56f06:crates/horizon-client/src/game/native_server.rs:385:    pub(super) fn native_sleep_ignored(&mut self, uuid: u128, ignored: bool) -> bool {
c9d474dd9116e84c243f8992d75c11314df56f06:crates/horizon-client/src/game/native_server.rs:480:    pub(super) fn native_run_command(&mut self, line: &str) -> bool {

== 5. NativeServerPlayer fields (no sleep/spawn/pose write) ==
pub(crate) struct NativeServerPlayer {
    pub(crate) uuid: u128,
    pub(crate) name: String,
    /// The operator level, 0 to 4 (`ops.json`; single player's owner with
    /// cheats on is 4).
    pub(crate) permission: u8,
    /// `minecraft:overworld`, `minecraft:the_nether`, `minecraft:the_end`.
    pub(crate) dimension: String,
    pub(crate) position: [f64; 3],
    /// 0 survival, 1 creative, 2 adventure, 3 spectator.
    pub(crate) game_mode: i32,
    /// Health points, 20 at full; 0 once dead. API 21.
    pub(crate) health: f32,
    /// Where they look, in degrees (`yRot`, `xRot`), as their last move told
    /// the server. API 49.
    pub(crate) yaw: f32,
    pub(crate) pitch: f32,
}

== 6. pin-internal sleep path is bed-name hard-coded ==
c9d474dd9116e84c243f8992d75c11314df56f06:crates/horizon-client/src/game.rs:13571:    fn try_sleep(&mut self, pos: [i32; 3]) {
c9d474dd9116e84c243f8992d75c11314df56f06:crates/horizon-client/src/game.rs:13760:    fn tick_sleep(&mut self) {
c9d474dd9116e84c243f8992d75c11314df56f06:crates/horizon-client/src/game.rs:13766:            let bed_gone = !self.res.facts().get(bed).name.ends_with("_bed");
c9d474dd9116e84c243f8992d75c11314df56f06:crates/horizon-client/src/game.rs:19955:fn tick_sleep_timer(timer: u32, sleeping: bool) -> u32 {
c9d474dd9116e84c243f8992d75c11314df56f06:crates/horizon-client/src/game/respawn_anchor.rs:588:    pub(super) fn bed_sets_spawn(&self, state: BlockStateId) -> bool {
c9d474dd9116e84c243f8992d75c11314df56f06:crates/horizon-client/src/game/respawn_anchor.rs-589-        self.dim == Dim::Overworld && &*self.res.facts().get(state).name != "minecraft:straw_bed"
c9d474dd9116e84c243f8992d75c11314df56f06:crates/horizon-client/src/game/respawn_anchor.rs-590-    }

== 7. sleep/bed/time/spawn hooks were added after the pin ==
native_player_bed_enter          a7f0d885a 2026-11-02 Native mods: player enters bed (native_player_bed_enter)
native_player_bed_leave          87f68a067 2026-11-02 Native mods: player leaves bed (native_player_bed_leave)
native_sleep_finished_time       a97ef0aaa 2026-11-02 Native mods: the time players wake at after sleeping (native_sleep_finished_time)
native_time_skip                 43a6fad78 2026-11-02 Native mods: server clock jump (native_time_skip)
native_player_spawn_point_set    a3786152e 2026-11-02 Native mods: player spawn point (native_player_spawn_point_set)
```

The five hooks above are **not at the pin**. They were added on 2026-11-02,
one day after the pin's 2026-11-01 tag, and are documented at HEAD
(`docs/native-mods.md:1376,1480-1483,1581-1588`). `tools/horizon.py` archives
`COMPAT["horizon_revision"]` (the pin) into the staged tree, so a port naming
them cannot be staged at this pin.

## 4. Why the pin-internal `try_sleep` is not a substitute

`Game::try_sleep` is a private method of `crate::game`; a support module could
reach it (the `break_block`/`add_player_effect` precedent), but it does not
implement Comforts' contract:

1. **It sets the spawn point.** For any Overworld bed whose block name is not
   exactly `minecraft:straw_bed`, `try_sleep` writes `respawn_config`
   (`game.rs:13643-13656`; `bed_sets_spawn`, `respawn_anchor.rs:588-590`).
   A mod block is named `comforts:sleeping_bag_red`, so it would set the spawn
   and show "Respawn point set" — the one thing the brief forbids.
2. **It treats a mod block as a gone bed.** `tick_sleep` wakes a sleeper whose
   block name does not end in `_bed` (`game.rs:13766`); `comforts:sleeping_bag_red`
   does not, so the player would be woken on the next tick. Renaming the id to
   `..._bed` still fails rule 1 (the name must be exactly `minecraft:straw_bed`).
3. **There is no sleep-attempt hook to call it from.** `use_item_on`/
   `right_click_block` see the click, not a sleep attempt; the engine's bed
   path is the only caller, and it only recognises a vanilla `AbstractBed`
   class (`interact.rs:344-365`). Calling `try_sleep` from a click would be a
   pin-internal workaround that fakes the sleep, which the brief rules out.
4. **The time-of-day rule and no-spawn rule are not parameters.** `try_sleep`
   is vanilla's night/thunder check; Comforts' day/night config and day-sleep
   wake-time logic would have to be reimplemented around a private method.

`native_run_command("time set day")` only moves the clock; it does not put the
player in bed, does not count as sleeping, and returns `false` inside a player
hook (`native_server.rs:480-492`). `native_sleep_ignored` only excludes a
player from the count; it cannot start sleep.

## 5. Why a content-only bed template is not a substitute

A content mod can declare a block from a bed template, but the block's id is
still in the mod's namespace (`comforts:sleeping_bag_red`). The pin's
no-spawn and destroy-on-leave rules are **name comparisons against the literal
`minecraft:straw_bed`**:

- `bed_sets_spawn` (`respawn_anchor.rs:588-590`),
- `free_left_beds` (`beds.rs:70-76`),
- `destroy_bed_on_use` (`respawn_anchor.rs:285`).

So a mod bed still sets the spawn. The 26.3 vanilla `minecraft:straw_bed`
(`StrawBedBlock`, `block-behaviours.json`) is the only no-spawn bed, and it is
not a mod block the port can register. There is no content path to Comforts'
contract.

## 6. What would unblock the port

A pin (or a named hook addition) exposing:

1. **Sleep start/leave** — `native_player_bed_enter(player, name, pos, cancel)`
   and `native_player_bed_leave(player, name, pos)` (HEAD: `a7f0d885a`,
   `87f68a067`).
2. **Time skip** — `native_sleep_finished_time(dimension, current, new_time)`
   and `native_time_skip(dimension, from, to, cause, cancel)` (HEAD:
   `a97ef0aaa`, `43a6fad78`).
3. **Spawn-point veto** — `native_player_spawn_point_set(player, name,
   dimension, pos, cause, cancel)` (HEAD: `a3786152e`).

With those, the sleeping-bag slice is implementable: place the block, let the
bed-enter hook accept a mod bed, skip the night through the time hooks, and
cancel the spawn-point hook for `comforts:*` beds. The hammock then adds
placement/rope support on top of the same sleep path. Until then the port is
`blocked(no-sleep-api)` and no sleep is faked.

## 7. Files read

Upstream (analysis only; clone removed after this file):

- `common/block/BaseComfortsBlock.java`, `SleepingBagBlock.java`,
  `HammockBlock.java`, `RopeAndNailBlock.java`
- `common/item/SleepingBagItem.java`, `HammockItem.java`, `BaseComfortsItem.java`
- `common/ComfortsEvents.java`, `ComfortsConfig.java`, `ComfortsRegistry.java`
- `common/capability/ISleepData.java`, `SleepDataImpl.java`
- `mixin/MixinSleepStatus.java`, `MixinServerSleepStatus.java`,
  `AccessorPlayer.java`
- `gradle.properties` (`9.0.5+1.21.1`)

Pin (read-only): `native-hooks.json`, `crates/horizon-client/src/game.rs`,
`crates/horizon-client/src/game/native_hooks.rs`,
`crates/horizon-client/src/game/native_server.rs`,
`crates/horizon-client/src/game/native_mobs.rs`,
`crates/horizon-client/src/game/respawn_anchor.rs`,
`crates/horizon-client/src/game/beds.rs`,
`crates/horizon-client/src/game/lan_actions.rs`,
`crates/horizon-world/src/level/interact.rs`, `docs/native-mods.md`.
