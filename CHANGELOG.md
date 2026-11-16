# Changelog

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
