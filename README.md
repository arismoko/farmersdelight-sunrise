# Farmer's Delight — Horizon port

A native [Horizon](https://github.com/arismoko/horizon) mod porting Farmer's Delight Refabricated 3.6.30
(Minecraft 26.3): cooking pot, stove, skillet, cutting board and knives, crops (cabbages, onions, tomatoes, rice),
rich soil and organic compost, mushroom colonies, wild crops and village additions, feasts and pies, all the meals,
drinks and pet foods with Nourishment and Comfort, storage blocks, baskets, cabinets, canvas signs, tatami, safety
net, rotten tomatoes, the Backstabbing enchantment, loot additions, advancements, the cooking pot recipe book, HUD
overlays, tooltips, config files and FD's 44 translations.

Version `3.6.30-horizon.2` (Farmer's Delight 3.6.30, Minecraft 26.3), built for Horizon `afe471cf15df56439f23cd991d0d4b85563a14fc`.

## Install

1. Copy `ported_farmersdelight.hmod` into the instance's `horizon-native-mods/` folder.
2. Build it for the instance with the launcher: `horizon-launcher native-build --instance <instance>`
   (native code runs with your rights; the launcher asks for `--trust-native-code` once).
3. Start the game. Settings are in `config/ported_farmersdelight-common.json` and
   `config/ported_farmersdelight-client.json` (FD's keys and layout; edits apply while the game runs, except the
   item-default keys, see below).

A dedicated server is exported with `horizon-launcher native-build <mod folder> --server-output <dir>`.

**Multiplayer:** every player needs the Horizon client with this same mod build. Clients without it are refused
("Your mods don't match this server's").

## Differences from Farmer's Delight

The port keeps FD's behaviour, quirks included. Where Horizon could not express something, the difference is
recorded as an accepted delta (A-nn) with its engine gap (E-nn) in `PROGRESS.md`. The ones a player may notice:

- the item-default config keys (stackable soups, rabbit stew buff, vanilla soup Nourishment, straw bed rename) have
  no effect: the engine reads config files after building item defaults (E-35, A47);
- the village config keys cannot switch off FD's village additions (E-26, A26);
- in single player, clicking a recipe in the cooking pot's recipe book does not place it (it works on servers;
  E-36, A48); the book's tabs are ordered Drinks, Meals, Misc (A49);
- the Comfort overlay ignores absorption hearts (A50); tooltip durations assume 20 ticks per second (A51);
- standing canvas signs open the text editor when placed (E-31, A34); a custom name is lost when a cabinet or
  basket is broken by something other than a player (E-30, A33);
- on page 2 of the creative inventory, the hovered tab's name can be drawn under a page-1 tab name (engine, E-39).

## For add-on authors

Depend on the exact base version, as the API may change in any release:

```toml
[dependencies]
ported_farmersdelight = "=3.6.30-horizon.2"
```

What stays stable while `fd_port_api` (a `[properties]` value in `horizon-mod.toml`, also
`ported_farmersdelight::api::API_VERSION`) is unchanged:

- the Rust module `api` for native add-ons (the launcher adds the base as the Cargo dependency
  `ported_farmersdelight`);
  - `api::feast`: FD's `FeastBlock` behaviour for an add-on's feast block (`register`, `take_serving`,
    `servings`, `decide`);
  - `api::wild_crop`: FD's `WildCropBlock` behaviour (`register`, `may_place_on`, `bonemeal_spread`,
    `suspicious_stew_effect`, …);
  - `api::config`: FD's boolean config keys by name (`common_bool`, `client_bool`), and
    `farmers_buy_fd_crops()` for add-ons written against upstream FD (always true here);
- the add-on tag overlay format (`tools/addon_overlays/`), which adds an add-on's ids to FD's own tags;
- the item tags `ported_farmersdelight:effect_tooltip` (food-effect tooltip lines) and
  `ported_farmersdelight:knife_items` (items that work as FD knives), and the base's load conditions.

A change that breaks any of these raises `fd_port_api`. Every base release changes the version, so add-ons are
re-released against it. FD's recipe types (`farmersdelight:cooking`, `farmersdelight:cutting`) accept recipes from
any namespace.

## Building from source

`horizon-mod.toml [native] horizon` and the `rev` in `Cargo.toml` name the same Horizon commit. Local checks
(Rust 1.99.1):

```sh
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
python3 tools/gen_decls.py --check && python3 tools/gen_worldgen.py --check
python3 tools/gen_food_tooltips.py --check
python3 tools/carry_fd.py --fd <FD checkout> --fapi <Fabric API checkout> --check
for t in tools/test_*.py; do python3 "$t"; done
cargo horizon-mod build      # target/horizon-mod/ported_farmersdelight.hmod
```

Carried Farmer's Delight data under `assets/` and `data/`, and Fabric API's convention tags under `data/c/tags/`, are
written by `tools/carry_fd.py`; never edit them by
hand. Block and item declarations come from `tools/decl/*.toml.in` through `tools/gen_decls.py`.

## Licence

This port's code is MIT (`LICENSE`). Farmer's Delight's data, assets and behaviour are MIT, © 2020 vectorwing
(`LICENSE-farmersdelight`). The `data/c/tags/` files come from Fabric API, Apache-2.0 (`LICENSE-fabric-api`). Credits
and the excluded LGPL files are listed in `NOTICE`.
