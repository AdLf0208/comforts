# Comforts — carried content (`.hmod`)

The data and asset half of the port: `comforts:sleeping_bed`, its block states,
models, item icon, the white-wool recipe, the head-only self-drop loot table,
the two `comforts:sleeping_bags` tags and a creative tab. Sleep behaviour
(hooks, config, time rules, spawn veto) belongs to the native bundle in `../`.

This `.hmod` is **not installed on its own**: slice 2 embeds it in the bundle
(`../mixins/comforts/data/comforts-content.hmod`) and the bundle writes it out
and loads it at start (the AE2 pattern), so `../comforts.hnmod` is the one
deliverable. The file here stays the build input and the byte-for-byte
comparison the e2e checks.

**LOCAL-ONLY.** LGPL-3.0-or-later (upstream), derivatives allowed, but never
published without the maintainer's approval (ARCHITECTURE.md §8).

## Provenance

| | |
|---|---|
| Upstream repository | <https://github.com/illusivesoulworks/comforts> |
| Branch / version | `1.21.1`, mod **9.0.5+1.21.1** |
| Commit | `5ee1465755dcbfc0e47b5d53a1c61ad0b70cf178` |
| Licence | LGPL-3.0-or-later (`COPYING`, `COPYING.LESSER`) |
| Files read | `common/src/main/java/.../ComfortsConfig.java`, `ComfortsEvents.java`, `block/BaseComfortsBlock.java`, `block/SleepingBagBlock.java`, `item/SleepingBagItem.java`; `common/src/main/resources/assets/comforts/{blockstates,models,textures/item,lang}`, `data/comforts/{tags,loot_table}`, `neoforge/src/generated/resources/data/comforts/recipe/*` |
| Analysis | `../FINDINGS.md`; the clone was analysis-only, never shipped |

Nothing was decompiled. The block's geometry is a port model: upstream draws the
bag from a Java `LayerDefinition` (a block-entity renderer, deep/client tier),
which a 26.3 data model cannot carry, so the port draws a 3 px flat bag body
with a small pillow using vanilla wool textures. The item icon is upstream's
`sleeping_bag_white.png` (LGPL, carried).

## What is included

```
content/
├── horizon-mod.toml                      id, API 0.2, side both, registries
├── assets/comforts/
│   ├── blockstates/sleeping_bed.json     8 variants: facing × part
│   ├── items/sleeping_bed.json           26.3 item model definition
│   ├── models/block/sleeping_bed_{head,foot}.json   the two halves
│   ├── models/item/sleeping_bed.json     item/generated over the bag icon
│   ├── textures/item/sleeping_bed.png    upstream sleeping_bag_white.png
│   └── lang/en_us.json                   block/item/tab + the two messages
└── data/comforts/
    ├── horizon/block/sleeping_bed.json   template minecraft:white_bed
    ├── horizon/creative_tab/comforts.json
    ├── loot_table/blocks/sleeping_bed.json   head-only self-drop
    ├── recipe/sleeping_bed.json          3 white wool → 1
    └── tags/{block,item}/sleeping_bags.json
```

## Why the id ends in `_bed`

The pin recognises a bed by its id (`tick_sleep` and the guest sleep tick wake a
sleeper whose block name does not end in `_bed`; `respawn_anchor::is_bed`). A
bed-shaped content block must therefore be named `…_bed`, so the port's block is
`comforts:sleeping_bed` instead of upstream's `comforts:sleeping_bag_<colour>`.
The display name is still "Sleeping Bag".

## Build

```sh
# Inside a build slot; builds the .hmod, checks it, syncs it into the bundle and
# packages the one .hnmod:
ports/tools/build-slot.sh run --job comforts-pack --port ports/comforts \
    --step package -- env CARGO_INCREMENTAL=0 bash -lc 'cd ports/comforts && tools/package.sh'
```

(or by hand, from `content/`, with the pin's own tool:
`cargo run --locked --offline --quiet --manifest-path
../../../ports/_shared/pins/horizon-1d63e0a/crates/cargo-horizon-mod/Cargo.toml
-- horizon-mod build`).

The delivered file is `content/comforts.hmod` (byte-identical to
`content/target/horizon-mod/comforts.hmod`): **4,379 B**, SHA-256
`66fe01d9a37d8804c7fc0a7797c40b77fc06d40d4b469023f98450b2b8580f4d`, 14 entries,
`comforts 0.1.0 (mod, API 0.2, both)`. Its `horizon-mod.toml` carries the
**port's** version `0.1.0` (the registry entry, the file and the release must
agree), while the upstream version is only in `ported-from` provenance.
