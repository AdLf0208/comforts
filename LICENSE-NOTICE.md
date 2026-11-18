# Licence notice — Comforts slice 1

- **Upstream:** Comforts 9.0.5 for Minecraft 1.21.1 / NeoForge
  (`illusiveSoulworks/comforts`, https://github.com/illusivesoulworks/comforts).
- **Licence as read:** `license = "LGPL-3.0-or-later"` in the pack jar's
  `META-INF/neoforge.mods.toml`
  (`ports/_shared/evidence/comforts/carry-work/comforts-neoforge-9.0.5+1.21.1.jar`), whose
  `LICENSE` file reads: "Copyright (C) 2017-2022 Illusive Soulworks … under the
  terms of the GNU Lesser General Public License … either version 3 of the
  License, or any later version". The jar also bundles `COPYING` (GPL v3) and
  `COPYING.LESSER` (the LGPL v3 additional permissions), both reproduced in
  [`LICENSE`](LICENSE).
- **Carried material:** the sleeping-bag block/item models, the sleeping-bag
  texture, the recipe, the tags, the creative tab and the English names.
- **Rust:** a clean-room rewrite against the pin's documented API; the upstream
  Java was read for behaviour only (see `FINDINGS.md`).
- **Rank:** 1 (copyleft). Publishable under **LGPL-3.0-or-later** with the
  complete corresponding source (this repository), and only with the
  maintainer's approval (ARCHITECTURE.md §8).
- **Package:** 0.2.0, the flat SDK format (mod API 0.2). This notice and the
  LGPL text travel inside `comforts.hmod`, which is the LGPL's condition on
  redistribution.
