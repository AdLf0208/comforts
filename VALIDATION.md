# Validation — Comforts slice 2 (pin `1d63e0a5`, one `.hnmod`)

> **HISTORICAL — the retired 0.1.0 bundle.** This table is the evidence of the `.hnmod`
> container and the staged ladder, both retired on 2026-11-18 (Horizon's mod-API merge #16941).
> Those tools no longer exist in this port and the artifact it names cannot load on a launcher
> ≥ 0.2.70. The SDK package's own ladder, package hash and entry list are in
> [`notes.md`](notes.md) §"Migration to the SDK format"; its in-game leg is an unrun gap because
> no `horizon-launcher` exists here. Keep this file for the behaviour transcript, not as the
> current verification.

Validated on Linux, Rust 1.97.0 (the port's `rust-version` is 1.95), against
the exact revisions in `compatibility.json`: Horizon
`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9` and horizon-mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57` (schema 6). Every build, test and
game step ran inside a `ports/tools/build-slot.sh` lease (`CARGO_INCREMENTAL=0`,
no `--target-dir`); compile steps forwarded to the second machine, the game leg
ran there too (headless, `--headless`, the forwarded e2e path).

| Check | Command | Result |
|---|---|---|
| runtime unit tests (standalone) | `cargo test --locked --offline` | **15 passed; 0 failed** |
| fmt | `cargo fmt --all` | clean |
| clippy | `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0, **0 warnings** |
| stage | `python3 tools/horizon.py stage` | exit 0, `horizon_revision 1d63e0a5…` |
| check (staged game, module compiled in) | `python3 tools/horizon.py check` | **`cargo_exit: 0`**, 0 warnings (`target/horizon-check.log`) |
| embedded tests | `python3 tools/horizon.py test` | **17 passed; 0 failed; 1240 filtered** — `game::comforts::tests` (15) + `game::comforts_state::tests` (2) |
| fast build | `python3 tools/horizon.py build` | **`cargo_exit: 0`** |
| content `.hmod` | `tools/package.sh` → pin's `cargo-horizon-mod build`/`check` | `comforts 0.1.0 (mod, API 0.2, both)`, **4,379 B**, SHA-256 `66fe01d9a37d8804c7fc0a7797c40b77fc06d40d4b469023f98450b2b8580f4d`, 14 entries, "no code: content only" |
| `.hnmod` | `tools/package.py` → `horizon-mod check … --kind native-mod` | **18,403 B**, SHA-256 `247d79370fbad02c14339adbc6b01354152460ef171e6c09fa9399f8e216b210`, 16 entries, `native-1/1d63e0a5…/5a64af93…`, **reproducible** (two consecutive runs, same sha256) |
| in-game e2e, embedded + control | `build-slot.sh e2e --job comforts-e2e --port ports/comforts -- tools/e2e.sh` | **PASS** + **`EXIT:0`** — `ports/_shared/evidence/comforts/e2e-embedded.log` |
| in-game e2e, selftest | `build-slot.sh e2e … -- env SELFTEST=1 tools/e2e.sh` | **FAIL** (the inverted no-spawn assertion) + **`EXIT:1`** — `…/e2e-embedded-selftest.log` |
| e2e log gate | `python3 ports/tools/check-e2e-logs.py comforts --strict` | exit 0, `logs=2 ok=2` |
| skills gate | `python3 ports/tools/check-skills.py --notes ports/comforts` | `SKILLS GATE: PASS` |
| upstream readiness | `python3 ports/tools/upstream_ready.py comforts --copyleft --include-blocked` | exit 0: licence copyleft (published under the same licence), artifact newer than its sources, `notes.md` Skills line, pin `1d63e0a5`, `e2e-embedded.log: PASS + EXIT:0` |

## What the game run showed

The full transcript is in the two evidence logs. The important lines:

```
== control: the same game with no embedded content file ==
  without the embedded file: 'Unknown item' and 'Unknown block type', the block at 2 64 0 is minecraft:stone
  embedded content: 4379 B at …/.horizon-cache/comforts-embedded-content.hmod, identical to content/comforts.hmod
  arena at 7 91 10 (world spawn 7 10)
== A/B: night sleep in the bag skips the night and sets no spawn ==
  placed: foot 7 91 17, head 7 91 18
  bag: asleep at [7,91,18] (head 7 91 18)
  bag: woke, day_time 18007 -> 24009, time_of_day 10
  no 'Respawn point set' line in the chat log
  player NBT respawn read: … no `respawn` key …
  respawned at … <0.5 from the world spawn>, <8.6 from the bag>
  the bag survived the night (break chance 0)
== C: the vanilla bed control does set the spawn ==
  'Respawn point set' seen (vanilla bed)
  the player NBT carries a respawn point (vanilla bed)
  respawned at … <1.1 from the bed, >4 from the world spawn>
== D: daytime sleep is refused (NIGHT), and the config is live ==
  noon with sleepingBagUse=NIGHT: refused (actionbar 'You can sleep only at night or during thunderstorms')
  noon with sleepingBagUse=DAY: asleep
  the day sleep skipped to the dusk (time_of_day 12009)
  night with sleepingBagUse=DAY: refused (the config drives the window)
== D2: restrictSleeping refuses the next sleep, and only while set ==
  restrictSleeping=true: refused (actionbar 'You are not sleepy yet')
  restrictSleeping=false: the same click slept
== E: sleepingBagBreakChance = 100 breaks the bag on waking ==
  the bag broke on waking (block at 3 91 15 is air)
  bag item entities on the floor after the sleep: 0
  the broken bag dropped nothing (as upstream's removeBlock)
== F: breaking the bag by hand returns one item ==
  the head dropped a bag on the ground
  bags in the inventory: 0 -> 1
PASS: …
EXIT:0
```

(The `<…>` figures are the distances the run printed; the assertions are the
`awk` comparisons in `tools/e2e.sh`, not these summaries.)

## Honesty notes

- **No Mojang files, no player data.** The run uses a scratch `HORIZON_HOME`,
  `XDG_DATA_HOME` and `XDG_CONFIG_HOME` under `/tmp/opencode`, a scratch game
  folder (the working directory), `--seed 20261106` and the prepared 26.3
  resources; it reads nothing of an installed instance.
- The e2e's "control" is two independent differentials: the vanilla bed at the
  same hour **does** set the respawn point (so a port that ignored
  `player_spawn_point_set` fails B while C passes), and the same binary with
  `.horizon-cache` blocked cannot see the ids at all (so the embedded leg is
  not passing on an installed copy).
- The selftest leg exists because a scenario that cannot fail is not evidence:
  it inverts assertion B and must exit 1, which it does.
- `tools/test_template.py` and `tools/test_stage.py` are the template's own
  fixtures (one `break_block` injection); this bundle has had 10+ injections
  since slice 1, so they are not this port's ladder and are not claimed as
  passing.
