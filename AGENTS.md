# Working on Comforts (native Horizon mod, SDK format)

This is a **flat SDK package** (mod API 0.2): `horizon-mod.toml` + `Cargo.toml` + `src/lib.rs`
(`impl mod_api::Hooks`) + `src/runtime.rs` (the pure rules and their tests) + `assets/` + `data/`.
Horizon retired the `.hnmod` container in the mod-API merge (#16941), so there is no
`compatibility.json`, `mixins/<id>/`, `horizon-mixin.toml`, plan/injection schema or
`rust-toolchain.toml` here, and none of them should be recreated. The retired tree is at
`../_shared/evidence/comforts/retired-hnmod-0.1.0/`.

> **Build slots (v2):** every build/test/lint step goes through the campaign runner
> (`ports/tools/build-slot.sh` from the campaign root). It **blocks in the kernel** until a slot
> is free, so **never poll and never background a build**; a timeout exits **75**. It does **not**
> `cd` into the port — wrap the command (`bash -c 'cd ports/comforts && …'`) and never pass
> `--target-dir`. Read `../_shared/build-slots-usage.md` before running anything.

```sh
ports/tools/build-slot.sh run --job comforts-sdk --port ports/comforts --step test \
    -- bash -c 'cd ports/comforts && CARGO_INCREMENTAL=0 cargo test --offline'
ports/tools/build-slot.sh run --job comforts-sdk --port ports/comforts --step lint \
    -- bash -c 'cd ports/comforts && cargo fmt --check && cargo clippy --all-targets -- -D warnings'
rm -f Cargo.lock
ports/tools/build-slot.sh run --job comforts-sdk --port ports/comforts --step build \
    -- bash -c 'cd ports/comforts && cargo horizon-mod build'
ports/tools/build-slot.sh run --job comforts-sdk --port ports/comforts --step check \
    -- bash -c 'cd ports/comforts && cargo horizon-mod check comforts.hmod'
```

- `cargo horizon-mod` fetches the pinned SDK into `.horizon/sdk/` (gitignored, never packed).
  `build` packs the `.hmod`; it does **not** compile against the game — that is
  `horizon-launcher native-build . --instance <instance> --trust-native-code`.
- **`src/runtime.rs` is byte-identical to the retired bundle's `runtime.rs` and must stay so**
  (sha256 `9959046ca5a8eb3ac67aa52b6f2841602cf8344c7017fd5e9f161f9d18009bc5`): every rule lives
  there, and `src/lib.rs` only converts SDK types to theirs. New rule → a test in `runtime.rs`;
  new glue → a private fn and a test in `lib.rs`.
- The six sleep moments are `Hooks` methods (`can_player_sleep`, `player_bed_enter`,
  `player_bed_leave`, `player_deep_sleep`, `player_set_spawn`, `sleep_finished_time`); the config
  is the four config hooks. `sleep_finished_time` is contract **v2**: it writes
  `ClockAdjustment::Absolute`, not `new_time`.
- Only the API the SDK publishes: `cargo horizon-mod hooks <text>` finds the exact method; never
  guess a name, and record an API gap in `notes.md` instead of faking one.
- The delivered package is `comforts.hmod` **in the port root**, never in `target/`; the LGPL
  text and `LICENSE-NOTICE.md` travel inside it.
- Keep the port separate from Horizon and horizon-mixins; never modify the pin checkouts, never
  commit binaries, exported sources or Minecraft assets, and never put Mojang files in the tree.
- `cargo fmt --check`, **never** `cargo fmt --all` (that formats the fetched SDK), and never let
  `cargo fmt` rewrite `runtime.rs`.
- `python3 ports/tools/check-skills.py --notes ports/comforts` must stay green, and `notes.md`
  records every claim with its command and output.
