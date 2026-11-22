# Changelog

## 3.6.30-sunrise.2 — 2026-11-22

Retarget from Sunrise `2d81263` to the Nightly `9425651` (2026-11-19, signed SDK; `retarget/NIGHTLY_PLAN.md`).
`3.6.30-sunrise.1` stays the build for `2d81263`.

- Engine API moves: block placement/survival methods take `LevelSide`; FD's loot types (`smoker_cook`,
  `copy_skillet`, `can_item_perform_ability`) are registered loot codecs; Nourishment and Comfort are effect parts;
  the cooking pot screen lives on its menu part; the tatami/mat partner is written in `set_placed_by`.
- Closer to FD (engine gaps fixed upstream): a player under Nourishment can always eat (A16 gone); Nourishment reads
  `naturalHealthRegeneration` (A15 gone); `enableVanillaCropCrates = false` removes the vanilla crate recipes again
  (A53 gone); rice has FD's block states (A22 gone); the skillet fires `minecraft:consume_item` (data-pack
  advancements see it). Single-player recipe-book clicks and the page-2 creative tab names work (engine fixes).
- Fixed: the stove and skillet no longer accept items from hoppers (FD exposes neither to automation); the cooking
  pot's meal-slot tooltip is the meal name and a gray "Served on" line, as FD.
- Add-on API unchanged (`fd_port_api = 1`); add-ons rebuild against this version.

## 3.6.30-sunrise.1 — 2026-11-17

Retarget from Horizon `afe471cf` to Sunrise `2d81263` (signed SDK; RETARGET_PLAN). Same content and behaviour,
except as listed.

- Manifest `sunrise-mod.toml`; packages are `.smod`; declarations under `data/farmersdelight/sunrise/`; engine data
  keys `sunrise:add_to` / `sunrise:climate`. The target lives in `tools/target.toml.in` (generators, `src/engine.rs`).
- Block containers are declared at registration (`block_container`), not by the removed `block_entity_container` hook.
- FD's recipe serializers are registered (`farmersdelight:cooking`, `farmersdelight:cutting` as read recipe types;
  `dough` and `food_serving` load as inert raw recipes, as before: E-12 still open).
- Closer to FD: the cooking pot's empty container slot shows FD's bowl icon; the recipe book's Misc tab shows its
  second icon (tomato sauce); the book toggle sits at FD's `height / 2 - 49`.
- New delta A53 (engine E-43): the vanilla crop crate recipes no longer carry the `enableVanillaCropCrates`
  condition (Sunrise's startup recipe load refuses unknown registered conditions); their advancements keep it.
- Add-on API unchanged (`fd_port_api = 1`); add-ons rebuild against this version.

## 3.6.30-horizon.3 — 2026-11-17

- Public add-on API `api::feast` (BX-04): `register` attaches FD's feast behaviour to an add-on's block;
  `take_serving`, `servings`, `decide` for blocks with their own interaction. FD's six feasts use it.
- Public add-on API `api::wild_crop` (BX-05): `register` gives an add-on block FD's wild-crop behaviour
  (ground tag, bone-meal spread, brown-mooshroom stew effect); the pieces are public too. FD's wild crops use it.
- Public add-on API `api::config` (BX-06): FD's boolean config keys by name; `farmers_buy_fd_crops()` is always
  true (FD Refabricated has no such key; its farmer trades are unconditional).
- Public add-on API `api::crop` (CD-06): vanilla crop growth speed and draws from the level's random, as FD's crops use.
- `api::feast::register_custom` (CD-11): a feast block with its own `useItemOn` (a `FeastBlock` subclass) keeps
  FD's survival, comparator and pathfinding and gets its own use and client prediction (`UseFn`, `PredictFn`).

## 3.6.30-horizon.2 — 2026-11-16

Add-on support; validated with More Delight (Horizon port) `26.9.16-horizon.1`.

- The version is now semver (`26.3-3.6.30-horizon.1` could not be parsed as a dependency requirement target);
  `[properties] fd_port_api = 1` and the public `api` module (README "For add-on authors").
- Fabric API convention tags (`c:`) shipped and merged with FD's own `c` tags.
- Add-on tag overlays: add-ons add their ids to FD-namespace tags through `tools/addon_overlays/` (optional
  entries); the More Delight overlay is included.
- New base-owned item tags: `ported_farmersdelight:knife_items` (FD knife behaviour by tag: carving, cake and pie
  slicing, straw and rice, scavenging, Backstabbing) and `ported_farmersdelight:effect_tooltip` (food-effect tooltip
  lines read from the item's consume effects).
- Cooking and cutting recipes and feasts from other namespaces are covered by regression fixtures.
- Engine gap recorded: creative tab names overlap on page 2 (E-39).

## 26.3-3.6.30-horizon.1 — 2026-11-16

First release: a native Horizon port of Farmer's Delight Refabricated 3.6.30 for Minecraft 26.3, built for
Horizon `afe471cf`.

- Cooking: stove, cooking pot (with recipe book), skillet (placed and held), cutting board and six knives; FD's
  28 cooking and 85 cutting recipes, Fortune on the board, dispensers working the board.
- Farming: cabbages, onions, tomatoes on ropes, rice, rich soil and rich farmland, organic compost, mushroom
  colonies, wild crops and FD's world generation, village compost heaps, FD crops on village farms, trades.
- Food: every meal, drink, feast and pie; Nourishment and Comfort; pet foods; vanilla soup changes.
- Blocks: crates, bales, rice bag, baskets, 13 cabinets, 68 canvas signs, tatami and mats, canvas rug, safety net,
  rope and rope fences, straw bed.
- Combat and loot: Backstabbing, rotten tomatoes, loot additions and scavenging, 21 advancements.
- Polish: config files, HUD overlays, tooltips, sounds and particles, creative tab, 44 languages.
- Known differences: see README and `PROGRESS.md` (deltas A1–A52, engine gaps E-01–E-38).
