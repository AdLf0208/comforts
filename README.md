# comforts (Horizon port)

A Horizon port of [comforts](illusivesoulworks/comforts) — LGPL-3.0-or-later.

**Status: first slices.** What is below is what the port actually does today.

## Implemented so far

- One block** `comforts:sleeping_bed` on the `minecraft:white_bed` template
- (bed class, `facing`/`part`/`occupied`), 3 px high (`shapes` and
- `collision_shapes`), hardness 0.1, wool sounds, `occludes: false`,
- head-only self-drop, 3 white wool → 1 recipe, `comforts:sleeping_bags`
- block/item tags, a creative tab and its item icon from upstream.
- Sleeping** uses the engine's bed path (`Class::AbstractBed` →
- `PlayerEffect::Sleep` → `try_sleep`), so reach, obstruction, the monster
- check, the sleep list, the stat and the `slept_in_bed` trigger are vanilla's.

## Not yet ported

- Block id** `comforts:sleeping_bed` (upstream `comforts:sleeping_bag_white`)
- because of the pin's `_bed` suffix rule (above). Only the plain white variant
- is carried; the other 15 dyed bags, the 16 hammocks, `rope_and_nail`, the
- sleeping-bag **item** (auto-use on placement) and the block-entity model are
- deferred.
- The bag's model is the port's**: upstream draws it from a Java
- `LayerDefinition` (a block-entity renderer, deep/client tier) which a data
- model cannot carry; the port draws a flat 3 px body and a pillow with vanilla

## Install

Build the bundle with a native-mod capable launcher, or add the file from the
registry once it is listed. Pinned to Horizon `1d63e0a` and horizon-mixins
`5a64af93` for game 26.3.

## Build from source

```sh
cargo test
python3 tools/horizon.py check --horizon <horizon checkout>
python3 tools/horizon.py build --horizon <horizon checkout>
python3 tools/package.py
```

## Credits and licence

Original mod: illusivesoulworks/comforts. Port: AdLf0208. Licence: LGPL-3.0-or-later — see [LICENSE](LICENSE);
the original notices are preserved.
