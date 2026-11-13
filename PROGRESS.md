# PROGRESS — farmersdelight-horizon

Append-only task log (CODEX_HANDOFF §6). Plan files: `/home/isubuki/Документы/claude` (substitutes
`/home/user/fd-port`); checkouts under its `sources/` (substitute `/home/user/arismoko/*` and
`/home/user/fd-port/sources/*`). Build pin `afe471cf15df56439f23cd991d0d4b85563a14fc` unless stated.

## M0-01 Verify workspace, pins and environment — PASS — 2026-11-13
- build pin: (not yet chosen) | mod commit: n/a
- did: cloned the three pinned checkouts; recorded tools and remote state.
- evidence: `../env-report.md` — SHAs c26fe59f…, 2c4100a3…, 1b85d66f… all match; rustc/cargo 1.99.1 (+rustfmt, clippy),
  Python 3.14.9, git 2.56.0, OpenJDK 27; 12 `prebuilt-*` tags; `main` = 0dde9dfa. MC 26.3 client jar: **yes**
  (user-supplied `minecraft-26.3-client.jar`, version.json id 26.3 / protocol 777 / world_version 5023);
  server jar: **yes** (user-supplied later, `minecraft-26.3-server.jar`, sha1 33680f5f…); display: yes (Wayland,
  Vulkan: Intel UHD + RTX 3060 Laptop).
- next: M0-02

## M0-02 Choose the build pin — PASS (route prebuilt, D2 step 2) — 2026-11-13
- build pin: afe471cf15df56439f23cd991d0d4b85563a14fc | mod commit: n/a
- did: no `prebuilt-*` tag descends from c26fe59 (ancestor check for all 12 after fetching main); took D2 step 2;
  ran PORT_PLAN §9 incl. step 5.
- evidence: release https://github.com/arismoko/horizon/releases/tag/prebuilt-afe471cf15df56439f23cd991d0d4b85563a14fc
  (pre-release, 2026-11-13T05:58:42Z, 5 assets incl. `horizon-prebuilt-v5-x86_64-unknown-linux-gnu.tar.gz` and
  `SHA256SUMS`; planning's "7 assets" counted the 2 source archives); diff `docs/repin/afe471cf…diff.txt`;
  46 M0–M1 cited symbols all present at afe471cf (`git grep -w`); `payload/registrar.rs` and
  `payload_transport.rs` present; hook catalog: no removals, c26fe59 only adds `can_drop_from_explosion`,
  `explosion_resistance`, `on_block_placed_by`, `on_destroyed_by_push_reaction`, `register_entity_layers`.
- deltas/probes: needs forward re-pin before M4-03 (Host::game_rule), M7-07 (on_block_placed_by), M9-03 (game_rule).
  mixins rev for afe471cf = `e59da9094f0c2fdda6c627fe9eff22dec6a906f0` (`horizon-mod-package/src/mixins_tool.rs:REV`).
- next: M0-03

## M0-07 Verify FD release publication — PASS — 2026-11-13
- did: Modrinth API (HTTP 200) lists `26.3-3.6.30` (release, 2026-10-24T17:58:38Z); jar downloaded to an isolated
  scratch dir, only `fabric.mod.json` read (version `26.3-3.6.30+refabricated`, MIT), never run.
- evidence: `../research/release-check.json`; sha512 099f7245…13bc matches API; SOURCE_LOCK.json `fd.release`.
- next: (independent)

## M0-03 Create the mod repository skeleton — PASS — 2026-11-13
## M0-04 Unit-test harness and first FakeHost test — PASS — 2026-11-13
- build pin: afe471cf | mod commit: 7562faf (one commit for both; fixes in 31f8c52)
- did: repo per §6.1 by hand (cargo-horizon-mod was still installing); manifest per plan with `[native] horizon =
  afe471cf…`, `mixins = e59da909…`; Cargo.lock seeded from the pin's lockfile (pruned by cargo to 54 packages,
  versions kept); CI = machine-sample CI + carry/decl/fixture drift checks. Tests: FakeHost `with_mods` +
  `dispatch_server_stopped`/`dispatch_server_about_to_start` clear the recipe tables; load-condition handler; table
  snapshot/replace.
- evidence: `cargo fmt --all --check` ok; `cargo clippy --all-targets --locked -- -D warnings` ok;
  `cargo test --locked`: 3 passed (`tests::lifecycle_hooks_clear_published_recipes`,
  `tests::crate_condition_follows_config_default`, `recipes::cache::tests::publish_replaces_and_snapshots_are_stable`).
  Toolchain 1.99.1.
- deltas/probes: **plan correction** — at afe471cf (and c26fe59, main 0dde9dfa) `LoadConditionDeclarations::
  register_handler` refuses a condition whose namespace differs from the *mod id*; owned namespaces do not apply.
  The condition is therefore `ported_farmersdelight:vanilla_crates_enabled` (carry rule changed, not files).
  `RegistrationContext::new` is not public to mods, so the registration dispatch is verified in game (M0-08), not
  in a unit test.
- next: M0-05

## M0-05 Deterministic FD carry script — PASS — 2026-11-13
- mod commit: 5747b7c (+ 31f8c52 rename)
- did: `tools/carry_fd.py` + `tools/carry_rules.toml.in`; byte copies, one rewrite rule; overlap and unknown
  `fabric:load_conditions` fail; stale-file removal; `--check`.
- evidence: count table — recipes 322, tag files 193 (c 75 / farmersdelight 60 / minecraft 58), loot tables 115,
  advancements 224, blockstates 135, block models 425, item models 188, items/ 188, png 486 (= 494 − 8 in
  gui/{jei,emi,eiv}), mcmeta 2, lang 44, ogg 14, sound events 20, worldgen 20, structures 5, villager trades 8,
  rewritten 6, total 2375. Second run byte-identical (manifest sha256 equal); `--check` ok; `tools/test_carry.py` 5 OK.
- next: M0-06

## M0-06 Declaration generator — PASS (generator); in-game admission in M0-08 — 2026-11-13
- mod commit: be435d4 (+ 31f8c52 rename)
- did: `tools/gen_decls.py` validates keys against the pin's `deny_unknown_fields` structs
  (`horizon-mod-package/src/content.rs`: BlockFile/BlockProperties/ItemFile/MobEffect/MenuType/CreativeTabFile/
  entity_type). M1 rows: flint_knife, minced_beef, beef_patty, cutting_board.
- evidence: `--check` ok on rerun; `tools/test_gen_decls.py` 5 OK; `cargo horizon-mod check` accepts the package
  (31f8c52: "ported_farmersdelight 0.1.0 (native-mod, format native-1/afe471cf…/e59da909…, for 26.3)").
- deltas/probes: plan correction — hardness/light/occludes/random_ticks/destroy_overlay live under `properties`,
  not at the top level as PORT_PLAN M0-06 lists. Fuel in 26.3 = `minecraft:cooking_fuel {burn_time,
  speed_multiplier}`. Tool tables are `*.toml.in` because the packager refuses `.toml`/`.rs` outside `src/`.
- next: M0-09

## M0-09 Parity fixture extractor — PASS — 2026-11-13
- mod commit: 9696fa3 (+ 31f8c52 rename)
- did: `tools/extract_fixtures.py`: cutting/cooking recipes + heat tags from FD data; foods (73 constants, incl.
  the record-form `KELP_ROLL`) and config defaults parsed from FD Java (MIT); knives transcribed
  (`tools/fixture_src/knives.toml.in`). `loot_rules` deferred to M8-05 (transcription task there).
- evidence: 117 cutting, 28 cooking, 73 foods, 6 knives, config 19 common + 3 client; `--check` ok;
  `tools/test_fixtures.py` 4 OK.
- deltas/probes: plan correction — the reference has **19** common config keys (plan says 18).
- next: M0-08

## M0-08 Package and load the mod in game — PASS (with notes) — 2026-11-13
- build pin: afe471cf | mod commit: cf409b3 (native build native-afe471c-85c629c17d21)
- did: no game release exists at afe471cf (launcher lists only nightlies ≤ 514ecf9; `prebuilt-*` tags are SDK-only), so:
  launcher built from source at the pin; `horizon-client`/ctl/prepare/server built (dev) at the pin; resources prepared
  from the user's Prism install + user-supplied server jar (Prism's Java 25); exporters compiled; wasm-std built;
  `horizon-launcher developer-install`; SHA256-verified `prebuilt-afe471cf` Linux SDK used via
  `HORIZON_LAUNCHER_DEVELOPER_SDK` for `native-build --trust-native-code` ("Built (optimized, prebuilt)"); played
  offline with `HORIZON_LAUNCHER_DEVELOPER_OFFLINE=1` (unsigned developer base; no account). All homes are scratch
  dirs under `../run/` (scripts: `../run/{env.sh,rebuild.sh,launch.sh,ctl,cmd}`).
- evidence: `../run/shots/latest-session.log` ("content from 1 mod(s)", no `E_NATIVE_*`, no reload refusal);
  `/give farmersdelight:flint_knife` etc. succeed; baseline `docs/baseline/m0-08-load-warnings.txt` (60 skipped
  cutting recipes naming undeclared ids; skillet item model type `farmersdelight:skillet_flip` unknown → M2).
  Mods-screen screenshot NOT taken (admission shown by log + working items instead).
- deltas/probes: (a) condition `ported_farmersdelight:vanilla_crates_enabled` registers; the client's own content
  load logs the 3 crate recipes as "unknown load conditions … left out" (client view outside a server reload); server
  side not yet verified by crafting a crate → open. (b) `backstabbing.json` admitted (no refusal logged). (c) staged
  carry mode NOT needed: the full carry loads with per-file skips.
- bug found and fixed: `Host::fluid_key` in `tags_updated` panicked the server thread ("unavailable during
  Reloading"); water is now read from the water block's fluid state in `state_for_placement`.
- next: M1-09

## M1-01…M1-08 slice — in-game results — 2026-11-13
- build pin: afe471cf | mod commit: cf409b3 | world FDTest (survival, seed 42), driven with horizon-ctl
- M1-01 PASS: `/give` of all four ids works; knife durability 131 (inventory report). Tooltip attribute lines not
  screenshotted. Unit fixtures for foods/knives exist (M0-09).
- M1-02 PARTIAL: placement `facing=north` with the player looking south (reference: opposite of placer) ✓; removing
  the supporting block pops the board, which drops itself and its contents (+1 board, +1 beef picked up, event log
  tick 6581/6590) ✓. Accepted delta **A9**: survival check approximated (non-air, non-`#minecraft:replaceable`
  below) — `can_survive`/`update_shape` get a level-only host without face-sturdiness/shape queries at the pin.
  NOT RUN: other three facings in game (unit-tested), waterlogged placement, bucket-waterlogging probe, fire test.
- M1-03 PASS: contents survive save & quit and a full process restart (taken back with empty hand after restart;
  still rendered after restart, `../run/shots/m1-after-restart.png`). One-slot ModContainer.
- M1-04 PASS (unit, 117 recipes, tool forms 51/25/15/15/6/3/2); in game `cutting/beef` used.
- M1-05 PASS: 3 beef inserted as a whole stack; knife cut → exactly 2 `minced_beef` dropped on the CCW side
  (x −1.875 from board centre −1.5 for facing north), knife 131→130, action bar "2 remaining..."/"1 remaining...",
  advancement "Watch Your Fingers" granted by the mod's `farmersdelight:use_cutting_board` trigger; empty-hand take
  returns the stack. New quirk **Q16**: reference `matches()` tests input and tool, so a wrong tool also shows
  "invalid item" (kept). Sounds NOT verified (audio off). Fortune bonus not read yet (M8-03). Block-item break sound
  path → M8-03.
- M1-06 PASS (single client): beef drawn flat, 2 layers for 2 items, 1 after a cut
  (`../run/shots/m1-06-beef-on-board.png`, `m1-cut-one-left.png`). **U-6 resolved: the client copy does NOT receive
  `items`**; the reference `Inventory` list is mirrored into the BE NBT and sent in the update tag. Upright
  block-item pose and LAN second client NOT RUN.
- M1-07 PARTIAL: models/textures/lang resolve (board, knife, beef rendered; English names in chat). Sounds unverified.
- M1-08 PASS: furnace and smoker turned minced beef into `beef_patty` (NBT-loaded with coal, done by tick 3902 from
  3685); campfire accepted minced beef by right-click and dropped a patty within its window (finished between ticks
  4006 and 4430 from 3690). Exact per-tick durations and furnace XP not measured.
- Eating: beef patty from food 0 / saturation 0 → food 4 / saturation 4.0 (vanilla clamps saturation to the food
  level). Crafting: flint knife and cutting board crafted from the carried recipes in a crafting table.
- next: M1-09 remaining items (see CHECKPOINT)

## M1-02 fixes from user play-testing — PASS (in game) — 2026-11-13
- reported: (1) placed board visibly rotates from the default facing to the right one; (2) breaking a waterlogged
  board removes the water; (3) beef stays in hand when put on the board.
- (1) cause: the client predicts placement from the block's template and never asks mods (`before_state_for_placement`
  only when `native_authoritative`). Fix: template `minecraft:stonecutter` (vanilla placement = opposite of placer,
  no BE, not flammable like FD's board); `facing` now comes from the template; `requires_tool=false`, oak-plank
  sounds, wood map colour. Its menu never opens (use_item_on answers every main-hand use; checked). Engine gap → E-14.
- (2) cause: engine removal reads the fluid from the template's facts, not the declared `waterlogged`. Fix:
  `remove_block` answers water for a waterlogged board; `update_shape` pop-off returns water. In game: player break →
  `minecraft:water[level=0]` + board dropped; support removed → water. Not covered: `destroyBlock` paths (explosion,
  `/setblock … destroy`). Engine gap → E-13.
- (3) not a bug: the player was in creative (`playerGameType` 1); FD keeps the hand stack in creative
  (`addItem(instabuild ? copy : stack)`, hand changed only `if (!player.isCreative())`). Survival removes it (checked).
- also: sound now on in the test env (`HORIZON_NO_AUDIO` removed from `run/launch.sh`; vanilla sounds imported from
  the user's Prism asset store with Mojang's 26.3 version JSON, sha1 11f1aefe… verified).
- note: boards placed before this change in the FDTest world kept loading (16 found after rebuild).
- follow-up (user report): water is missing for one tick on the client when a waterlogged board is placed or broken.
  Cause: client prediction uses the stonecutter template (no water on removal; no `waterlogged` on placement); the
  server's correct state follows one tick later. **Decision (user, 2026-11-13): accept until E-13/E-14 land**; no
  fork change. Recorded as accepted delta A10.

## M1-02/M1-07 remaining probes — PASS — 2026-11-13
- four facings: looking north/east/west/south gives facing south/west/east/north (opposite of placer) for boards placed
  by horizon-ctl with an explicit look; top-down screenshot `../run/shots/m1-four-facings.png`.
- bucket probe: emptying a water bucket on a board puts the bucket *on the board* (useItemOn runs first and the bucket
  fits), as in the reference; bucket-waterlogging is therefore not reachable in either. Waterlogging by placing into
  water works (earlier).
- upright block item: a pumpkin stands as a small block on the board (`../run/shots/m1-pumpkin-upright.png`).
- sounds: verified through subtitles (audio on): "Item placed on board", "Knife cuts" (`m1-09/03`, `m1-09/06`).
- crate load condition, server side: a world datapack recipe gated by
  `ported_farmersdelight:vanilla_crates_enabled` (9 dirt → diamond) loaded after `/reload` ("Unlocked 1 recipe")
  and crafted. The FD crate recipes themselves stay out until `farmersdelight:*_crate` items are declared (M7).
- side note: once, closing a crafting table right after scripted clicks lost its 9 grid items (not reproduced; engine
  or scripting race, not mod code).

## M1-09 M1 in-game demonstration — PASS — 2026-11-13
- build pin: afe471cf | mod commit: d2db874 | native-afe471c-f016ff3649f3 | fresh survival world "M1Demo" (seed 1234,
  peaceful), driven with horizon-ctl; recordings `../run/horizon-home/instances/fd-port/recordings/*m1-09-demo*`;
  screenshots + logs `../run/shots/m1-09/`.
- (1) knife and board crafted in a crafting table from carried recipes (01, 02); materials given with /give.
- (2) cutting: each raw beef → exactly 2 minced_beef, knife 131→130→129 (1 per cut), action bar "1 remaining...",
  "Knife cuts" subtitle, "Watch Your Fingers" toast (06).
- (3) beef visibly rendered on the board before cutting (03), "Item placed on board" subtitle.
- (4) minced beef → beef_patty in a placed furnace and smoker (loaded via their menus with coal) and on a campfire;
  furnace/smoker output gave XP (XpP 0 → 0.286) (07).
- (5) eating: food 8 → 16 and saturation 0 → 12.8 for two patties (+4 / +6.4 each) on Easy (peaceful refills food).
- (6) board contents survive save & quit to title + reopen (04) and a full process restart (05), still rendered.
- (7) real single-player path (Horizon client at the build pin, packaged mod, integrated server); mod server hooks
  run on the integrated server thread (recipe tables built there; the earlier fluid_key panic was reported on
  'Server thread'). Log warnings only from the M0-08 baseline.
- not covered: LAN second client (optional for M1), measured per-tick cook durations.
- env: `run/play.sh` added for the user to play with a captured mouse; `run/launch.sh` (agent, `--control`) never
  captures the pointer at afe471cf (`unclaimed_window` is never cleared, also on main 2026-11-13).
- next: M2-01

## M2-01 Heat module — PASS (unit) — 2026-11-13
- mod commit: 2026 series (cd17057) | `src/heat.rs`: HeatableBlockEntity rule over a lookup; 12-case table test; carried
  tags checked. In-game proof is due with the placed skillet (M2-09) and pot heat icon (M3-05), per plan.

## M2-02 Stove block — PASS — 2026-11-13
- template `minecraft:stonecutter` (native facing placed opposite; no BE), full-cube shape, bricks' properties,
  declared `lit` default true, `state_light` lit→13; empty-hand template use (stonecutter menu) answered Pass.
- in game (M1Demo): placed → `lit=true` (Q7); block light at the stove = 13; iron shovel → unlit, durability −1;
  flint & steel → lit, durability −1; water bucket → unlit and an empty bucket returned.
- found: FD relies on **Fabric API convention tags** (`c:buckets/water`, `c:tools/shear`, crops, dyes, ingots …)
  that Horizon does not ship. The mod now generates the 28 item tags FD's data needs
  (`tools/conventional/item_tags.toml.in` → `data/c/tags/item`, `tools/gen_conventional_tags.py --check` in CI);
  skipped cutting recipes 60 → 57. Biome convention tags left for M6.

## M2-03 Stove block entity — PASS — 2026-11-13
- 6 one-item slots, campfire recipes via `LevelRef::recipe_for`; NBT `CookingTimes`=totals,
  `CookingTotalTimes`=progress (Q4); `Inventory` mirrored for the client; ticks always.
- in game: 6 beef accepted, 7th refused; 6 cooked beef after 600 ticks each; shovel mid-cook → progress −2/tick
  (342 → 260 in ~2 s), relight resumes; save & quit + reopen mid-cook keeps progress and continues (no instant cook);
  bottom slab above → items drop; placing food under a bottom slab refused. Top slab: unit-tested only
  (`covers_grill`). No XP (Q12).

## M2-04 Stove client — PARTIAL — 2026-11-13
- renderer: 6 items flat in the 3×2 grid on the lit top (`../run/shots/m2-stove-6-beef.png`, facing north).
  Client tick: crackle 1/10, smoke + flame at the front, item steam 20%×3, lit only (code; particles not yet
  screenshotted, other three facings not yet checked).

## M2-05 Stove burn — PASS (with delta A11) — 2026-11-13
- `block_step_on` has no host and `entity_inside` cannot hurt, so the stove's tick hurts living, on-ground,
  non-sneaking entities meeting the grilling area above (FD's condition), 1.0 per hit.
- **E-15**: mod damage types are not loaded (damage catalog built without mod packs; `/damage …
  farmersdelight:stove_burn` → "invulnerable"; tags naming it "unresolved"). Burn uses `minecraft:hot_floor`
  (same `is_fire`/`burn_from_stepping` tags) → accepted delta **A11** (death message differs).
- in game: standing on lit stove → hurt 1.0; sneaking 4 s → 0; unlit → 0.
- engine observation: `/damage` with `generic` or `hot_floor` (both in `#no_knockback`) still knocks the player
  ~1 block at the pin → being burned also flings the player off the stove. Not mod code; candidate engine bug report.
- next: M2-06 (skillet item)

## M2-06 Skillet item — PASS (player-victim sound pending LAN) — 2026-11-13
- mod commit: b4af55b | item `farmersdelight:skillet` declared as a plain item (stack 1, 250 durability, repair
  `#minecraft:iron_tool_materials`, enchantable 14, damage +7, speed −3.1, knockback +1, Tool([],1,1),
  Weapon(1)); block `farmersdelight:skillet` (template `minecraft:stone_stairs`: native facing = look
  direction and native waterlogged, as SkilletBlock; `item=false`, `block_item` = the skillet).
- in game: 250 durability; each zombie hit costs **2** (250→248→246): vanilla runs `Item.postHurtEnemy`
  (FD: −1) and then `Weapon` (−1) — new quirk **Q17** (plan said 1). Normal right-click on the ground places
  nothing; sneak-right-click places the skillet facing the look direction (`m2-skillet-placed.png`).
- attack sound only when the victim is a player (Q2): implemented in `attack_entity` (charge read before the
  swing resets it); needs a second player → M2-10 LAN.

## M2-07 Held cooking — PASS — 2026-11-13
- the engine only lets the client enter an item use itself: the skillet answers a use duration always and
  `living_entity_use_item_start` (both sides) checks heat (3×3×3, tag only, or on fire), the off-hand campfire
  recipe and water, sets FD's `getSkilletCookingTime` (Mth.clamp = min(max(v,60),orig)) or cancels; the server
  stores one unit in `farmersdelight:skillet_ingredient` + `cooking_time_length` (component types registered).
- in game next to a campfire: early release (3 s) returns the beef (off hand back to 3); full hold → cooked beef.
  Quirk **Q18** (kept, from source): FD Refabricated consumes 2 raw items per cooked one (the start leaves the
  other hand at count−1, `finishUsingItem` shrinks it again).
- not checked in game: underwater refusal, Fire Aspect times (unit table: 120/80/60), bar colour.

## M2-08 Skillet visuals — PARTIAL (degraded A12) — 2026-11-13
- FD's item definition uses a custom node with an animated Java renderer. Horizon mod nodes only choose among
  named child models, ids `<mod id>:<path>` → carry **override** rule (approved exception) restates
  `items/skillet.json` with `ported_farmersdelight:skillet_flip` + `models.cooking`; `is_cooking` property
  = has ingredient. Skillet now renders held (`m2-skillet-held-*.png`); no "unknown item model type" warning.
- omitted (delta **A12**, engine task **E-16** animated mod item renderer): the flip animation; hence the
  `flip_skillet` payload/arm pose are not implemented yet (cosmetic only). Sizzle sounds: not yet.
- next: M2-09 (placed skillet block entity)
