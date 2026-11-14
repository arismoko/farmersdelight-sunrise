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

## M2-09 Placed skillet — PASS — 2026-11-13
- mod commit: e92ec37 | block entity: one slot (whole stack), `CookTime`/`CookTimeTotal`/`Skillet`, `Inventory`
  mirrored; heated (conductors count) +1/tick, result toward `facing.getClockWise()`, one unit used; cold −2/tick;
  waterlogged → contents drop; `support` recomputed on vertical neighbour change (tray heat sources).
- in game: sneak-placed on a campfire → `support=true`; 8 beef in (whole stack, "Skillet hisses" add-food sound,
  steam particles) → 8 cooked beef after 8 × 80 ticks with Fire Aspect I (`m2-skillet-8-beef.png`); an
  enchanted, damaged skillet broken → drops with `damage 10` + Fire Aspect I; water bucket → waterlogged
  (native, stairs template) and the beef drops out.
- engine notes: (1) the client predicts block clicks with the template and also sends an off-hand round, which
  emptied the pan right after filling it; FD's main-hand `useItemOn` never passes, so the part ignores the off
  hand (parity). (2) Horizon's loot context has no block-entity data and the block entity is gone when
  `loot_drops` runs: `farmersdelight:copy_skillet` is a no-op loot function and the placed stack is recorded in
  `pre_remove_side_effects` and applied in `loot_drops`.
- next: M2-10 (demo incl. save/restart mid-cook and LAN second client)

## M2-10 multiplayer part — PARTIAL — 2026-11-13
- setup (no account): dedicated server exported with the mod (`native-build --server-output run/server-engine`,
  server SDK prebuilt-afe471cf, sha256 verified), `run/fd-server` offline mode bound to 127.0.0.1:25570; two
  clients run directly from a copy of the modded build (`run/mp/client.sh A Alice`, `B Bob`; ctl wrappers
  `run/mp/ctlA|ctlB`, console `run/mp/sv`). Music 0, master volume 25% on both (user request).
- PASS: second client (Bob) sees Alice's stove items on the lit stove, its flames/smoke, and the placed skillet's
  contents (beef in the pan, steam) (`run/shots/m2-10-bob-view*.png`).
- NOT RUN yet: skillet attack sound with a player victim (FD's attack sounds have no subtitle, so it needs a
  listen check), save/restart mid-cook on the server, single-player recorded demo run.

## M2-10 multiplayer part 2 — PASS — 2026-11-14
- skillet attack sound, player victim (Q2): Bob's clientbound packets recorded (`studio record`) while Alice
  hit him: 2 full-charge hits → 2 × `farmersdelight:item.skillet.attack.strong`; 3 rapid hits → 1 strong +
  2 `…attack.weak` (charge read before the swing). Evidence: `run/mp/B/game/recordings/skillet{pvp,weak2}_*.mcpr`.
- save/restart mid-cook on the dedicated server (`tick freeze` → `data get block` → `horizon-server stop` → run
  → reconnect): stove `CookingTotalTimes` [305,283,261] and later [472,450,428] identical after restart (two
  restarts); placed skillet `CookTime 40/120`, 7 beef identical. After unfreeze both advance at the normal rate
  (+6 ticks each), no instant cook. Bob sees the stove's beef after the restart (`run/shots/m2-10-bob-after-restart.png`).
- tool note: `horizon-ctl interact` refuses the placed skillet ("no face visible") from a standing eye height;
  `look-at` + `mouse right click` works (ctl visibility check, not mod behaviour).
- left for M2-10: recorded single-player demo run. Flip pose not checkable (A12 / E-16).

## M2-10 M2 demo (single player) — PASS — 2026-11-14 — **M2 closed**
- build pin: afe471cf | mod commit: e84a627 | native-afe471c-8f84ec89f8f0 | fresh survival world "M2Demo", horizon-ctl;
  recordings `../run/horizon-home/instances/fd-port/recordings/*m2-10-demo*`; screenshots + logs `../run/shots/m2-10/`.
- **bug found and fixed (e84a627):** in survival, beef put into a placed skillet came straight back into the *off*
  hand. `use_item_on`/`use_without_item` called `interact` directly, bypassing the main-hand-only filter, so the
  client's off-hand round (main hand now empty) ran the empty-hand take. Creative hid it (hand stack kept), which
  is why M2-09 and the LAN run passed. After the fix: 9 beef stay in the pan; empty main hand takes the stack back
  into the main hand, off hand untouched.
- stove: 6 beef accepted, 7th refused (01); all 6 cook and pop out (12 cooked beef collected over two loads).
- placed skillet on a campfire (`support=true`), cooks one unit per 120 ticks, result ejected east (facing north).
- held cooking near the campfire, looking at the sky: off hand 3 → 1 → 0, one cooked beef per 2 raw (Q18), early
  release returns the unit (03). Note for scripts: with the crosshair on the placed skillet the click goes to the
  block ("This can't be cooked...").
- stove burn: hurt 1.0 standing on the lit stove (04), 0 hurts in 3 s sneaking.
- save & quit to title + reopen mid-cook (`tick freeze` snapshots): stove [251…241] → [267…257], skillet
  118/120 (8 beef) → 14/120 (7 beef): both +16 ticks, the exact load-to-freeze gap; no reset, no instant cook (05, 06).
- full process restart: stove [267…257] → [282…272], skillet 14 → 29: both +15 ticks (07); then cooking finished
  normally (08). Logs `session{1,2}-latest.log`: no panics; only the known crate load-condition client lines.
- M2 open items carried forward: flip pose / sizzle (A12, E-16), stove other three facings not screenshotted,
  skillet underwater refusal and Fire Aspect times not checked in game (unit-tested).
- next: M3-01 (cooking codec)

## M3-01 Cooking codec — PASS (unit) — 2026-11-14
- mod commit: 554c1bb | `src/recipes/cooking.rs`: codec per `CookingPotRecipe.Serializer` (`group` "", `recipe_book_tab`
  meals/drinks/misc default misc, `ingredients` 1–6 with `fabric:any` flattened to alternatives, `result`,
  optional `container`, `experience` 0, `cookingtime` 200); table rebuilt with cutting on every data load.
- matcher: Refabricated's `RecipeWrapper` counts non-empty input slots and feeds `StackedItemContents` **one unit
  per slot** (`accountStack(stack, 1)`), so a match = filled-slot count equals ingredient count + perfect bipartite
  matching (Kuhn). Stack sizes never matter; two identical ingredients need two slots. `find` tries the last
  matched recipe first, then file order (`RecipeManager.CachedCheck`), none when all inputs are empty (`hasInput`).
- unit (9 new, 40 total): all 28 parse (sizes 1/2/3/4/5/6 = 1/4/6/15/1/1; tabs meals 20, drinks 2, misc 6); the
  3 `fabric:any` files (bone_broth, cabbage_rolls, dumplings) flatten to 4 alternatives; count rule; augmenting
  path; duplicates; stale cache; build skips empty ingredients and unknown results.
- in game (load only): 3 of 28 resolve (beetroot_soup, mushroom_stew, rabbit_stew); 25 skipped with a logged
  reason, all for ids or `c:` tags whose FD items are declared later (crops M5, meals M7). They load once those
  exist; M3-03's "each of the 28 recipes cooks" can only be fully checked after M5/M7 (until then: the 3 vanilla ones).
- next: M3-02 (pot block)

## M3-02…M3-09 Cooking pot — CODE DONE, in-game testing IN PROGRESS (interrupted) — 2026-11-14
- mod commit: 064e4f3 | `src/blocks/cooking_pot.rs` (block, BE tick, menu, screen, XP, item form, automation, client
  FX), decls `tools/decl/blocks.toml.in` [cooking_pot] (template stonecutter, declared `support`
  none/handle/tray + `waterlogged`, max_stack 1) and new `tools/decl/menus.toml.in` (menu `cooking_pot`: 9 slots,
  2 data slots, result slot 8). clippy clean, 49 unit tests (9 new pot tests: cool clamp, meal-slot fit ≥64,
  meal→output, servings, comparator formula, XP rounding, arrow width, remainder overrides, shift-click merge).
- design notes / engine gaps found:
  - meal slot 6 is **hidden** in the screen (`Screen.slots[6] = None`) and drawn from the BE's synced `Inventory`
    (via `screen_pos()`), so it can be neither placed into nor taken (CookingPotMealSlot); `menu_slot_rules` also
    refuses placement.
  - **E-17 (new)**: `menu_quick_move` cannot empty an inventory slot (API `ItemStack` has no 0 count; an unchanged
    stack is ignored) → FD routing (serving containers → slot 7, rest → 0–5) is applied only when part of the
    stack stays behind; a stack that moves whole falls back to the game's routing.
  - **U-11** (no take hook): XP/recipes on output take = shift-click from slot 8 (in `menu_quick_move`), or a tick
    diff of the output count awarded to the player who opened the pot (a hopper pulling while the menu is open
    would also award: small delta, to record as A13).
  - comparator: `analog_output` gets a level host with no BE read → the tick computes the signal, caches it per
    position, and calls `update_neighbors(OutputSignal)` when it changes.
  - item form: the loot table's `copy_components` needs the BE → (as the skillet) contents/meal recorded in
    `pre_remove_side_effects`, drop components `farmersdelight:meal`/`container` + `minecraft:block_entity_data`
    {Items:[meal at Slot 6], Container} set in `loot_drops` (placement should merge it back — UNTESTED).
  - per-tick sync while the container slot is filled (FD does it) dropped: sync only on real moves.
  - not done: food_serving crafting (E-12 fallback: serving by right-click/menu only), dough recipe (wheat_dough not
    declared yet), item tooltip image (CookingPotTooltip), custom name on the BE.
- in game so far (world M2Demo, survival): pot on a campfire → `support=tray`, facing opposite the placer ✓;
  "Dinner's Served!" advancement ✓; empty-hand right-click opens the menu ✓; **full GUI texture works**
  (`farmersdelight:gui/cooking_pot` → U-10 resolved, no sprite fallback needed), heat icon shown over the campfire ✓,
  title/inventory labels ✓, "Cooking Pot boils" subtitle ✓ (`run/shots/m3-pot-menu-empty.png`).
- NEXT (resume here): in M2Demo the player has 4 brown + 4 red mushrooms and 3 bowls (pot at -34 65 -7 on a campfire);
  put a mushroom in two input slots (click-slot 300/301) → expect mushroom_stew after 200 ticks in the meal display,
  then bowl in slot 7 → stew in output; check arrow, data slots, shift-click routes, XP on take, comparator, handle
  toggle (sneak + empty hand), hanging pot under a ceiling, hopper faces (top → inputs, side → container, bottom ←
  output), break with meal → item keeps meal → re-place restores it, save/restart mid-cook, then M3-10 demo
  (vegetable soup needs M5 items → demo with mushroom/beetroot/rabbit stew instead, note the deviation) + LAN guest.

## M3 cooking pot — in-game results — PASS (with deltas) — 2026-11-14 — **M3 closed**
- mod commit: 77462a4 | build native-afe471c-a6ba635b96c7 | world M2Demo (survival) + dedicated server (Alice/Bob);
  recordings `../run/horizon-home/instances/fd-port/recordings/*m3-10-demo*`; shots `../run/shots/m3-*.png`,
  `../run/shots/m3-10/` (+ session logs).
- fixes made during testing (commit before 77462a4): (1) **meal kept in the saved data** (`Meal`), not container slot 6:
  the engine drops a mod container's slots after `pre_remove_side_effects`, so the meal also fell out loose
  beside the pot item that carries it (duplication); slot 6 stays empty/hidden. (2) take-XP diff compares the output
  at the **start** of the tick (the same tick's serving refilled it and hid the take). (3) `open_menu` returns
  false although the menu opens → the opener is recorded regardless. (4) serving containers refused in ingredient
  slots (`menu_slot_rules` via a mod recipe property set `farmersdelight:serving_containers`, items = the carried tag's
  three, since the loop/data-load contexts have no item tags) → a whole shift-clicked bowl stack now reaches the
  container slot through the game's routing (E-17 workaround). Accepted delta **A13**: an empty bowl/bottle/bucket
  cannot be put into an ingredient slot by hand (no recipe uses one).
- M3-01/02: pot on a campfire → `support=tray`, facing opposite the placer; placed against a ceiling → `handle`;
  sneak + empty hand toggles handle ↔ tray (lantern sound); "Dinner's Served!" advancement.
- M3-03: mushroom stew (2 slots) cooks in 200 ticks; meal in the display, `Container` = bowl, `RecipesUsed` counted;
  inputs shrink one each; with bowls in slot 7 meals fill the output (one bowl each; stew stacks to 1 at this point,
  FD's stackable-soups config is M4); unheated/no-recipe decay per the reference (unit).
- M3-04: menu 9+36 slots, shift-click output → inventory, inventory → inputs / bowls → container slot; meal slot
  hidden (neither placeable nor takeable).
- M3-05: **full GUI texture works** (`farmersdelight:gui/cooking_pot`; U-10 resolved, no sprite fallback); heat icon
  when heated; arrow width = cookTime·24/total (+1) measured 3 px at 21/200, 18 px at 144/200; meal drawn with count.
  Tooltips (heated / meal + "Served on") implemented, **not hover-tested** (ctl cannot hover).
- M3-06: XP + recipe award on shift-click take (2 xp for 2 stews) and on plain-click take (+1, tracker cleared);
  breaking the pot pops the pending XP at the block (+1). Delta **A14**: a hopper pulling the output while a player
  has the menu open also counts as that player's take (no take hook, U-11).
- M3-07: the dropped pot carries `farmersdelight:meal` + `farmersdelight:container` + `minecraft:block_entity_data`
  {Meal, Container}; re-placing restores the meal (served from it with a held bowl afterwards). The other slots drop
  once (5 carrots), the meal never drops loose. **Not done** (carried forward): tooltip image (CookingPotTooltip),
  `food_serving` crafting (E-12 fallback: serving by right-click/menu works), `dough` (wheat_dough not declared yet),
  custom name.
- M3-08: hopper above → ingredient slots; side hopper → container slot; hopper below an unheated pot pulls only the
  output (carrots stay). Comparator: FD formula (unit) and in game 1 for 12 cooked beef + 1 bowl; the signal is cached
  by the tick and pushed with `update_neighbors(OutputSignal)` (a comparator placed by /setblock reads 0 until the
  next content change).
- M3-09: `farmersdelight:steam`, `star`, `sparkle` particle types registered; boil sound ("Cooking Pot boils"
  subtitle) while heated; bubble/steam particles coded (not separately screenshotted).
- M3-10 demo: pot **crafted** from the carried recipe (bucket left in the grid); beetroot soup cooked and served into
  bowls; mushroom stew **fully automated** (top hopper, bowls in slot 7): 3 cooked; save & quit + reopen mid-cook
  138 → 153 and full restart 153 → 168 (+15 = load-to-freeze gap), all slots identical. **Deviation:** vegetable
  soup needs M5 crops, so the demo uses beetroot soup / mushroom stew. Hopper insertion puts all of one item into the
  first slot (FD's Transfer insert does the same), so 3×beetroot cannot be automated in FD either.
- LAN (dedicated server, server engine re-exported): Alice and Bob open the same pot; both arrows 17 px at 148–150/200;
  5 rounds of simultaneous shift-clicks on the output: 3 stews cooked = 3 received (Alice 3, Bob 0), 3 bowls used —
  nothing duplicated or lost.
- engine gaps recorded: **E-17** quick move cannot empty an inventory slot (workaround above); U-11 confirmed (no
  menu take hook); `analog_output`'s level host has no block-entity read (cache workaround).
- next: M4-01

## M4 foods, drinks, effects, feeding — 2026-11-14
- mod commits: 1188042, b4a1a64 (code), 3360cfc (log) | decls: 79 food/drink rows generated once from ModItems.java + tests/fixtures/foods.json
  (`tools/decl/items.toml.in`, each row cites its FoodValues constant and consumable), effects
  `tools/decl/effects.toml.in` (nourishment 0xF3B300, comfort 14545909, beneficial), materials straw/canvas/tree_bark,
  compost tiers on every declared compostable (fixture `tests/fixtures/compostables.json`, 39 ids incl. straw_bed).
  Tests: `tools/test_food_decls.py` (77 food items = their FoodValues constant; bowl/bottle foods return the container
  via `minecraft:use_remainder` and stack 16; Q3 onion soup = SHEPHERDS_PIE), `tools/test_compost_fuel_decls.py`
  (re-run at every milestone), Rust units for Nourishment/Comfort rules, numeric payloads, vanilla soup overrides,
  pet foods (55 Rust + 19 Python tests).
- M4-01 in game (survival, M2Demo): hamburger +11 food; beef stew → Nourishment 3600 + bowl back; apple cider (always
  edible, at full food) → Absorption 1200 + bottle; glow berry custard → bottle; cooked rice stacks to 16; full
  player refuses cake slice / chicken cuts (not always edible). 23 of 28 cooking recipes now resolve (rest need rice,
  M5, and the stuffed pumpkin block, M7). `onion` is declared as a plain food item; M5 makes it the crop's block item.
- M4-02: hot cocoa removed Poison but not Speed (harmful only); milk bottle removed one random effect; melon juice +2.0
  HP (measured with natural regeneration off); melon popsicle put the player out (Fire 291 → -20). Fix made: data-driven
  payloads keep JSON numbers in the narrowest NBT type (`2.0` arrives as a byte), so the heal amount reads any numeric
  tag. Q13: the `ignored/*` effect tags are empty in FD and mob-effect tags cannot be queried at the pin, so no
  exclusion is applied (a datapack adding to them is not honoured: delta A18).
- M4-03 Nourishment (via `apply_effect_tick`: `add_exhaustion(-min(e,4))`, which the engine applies through
  `cause_food_exhaustion`, i.e. exactly FD's call): server readings — nourished player under Hunger 251 for 10 s: food 9 /
  saturation 0 unchanged; same exhaustion without Nourishment for 2 s: food 17 → 9. Residual delta: exhaustion above
  4.0 per tick is not possible from vanilla sources, so none measured. The `naturalHealthRegeneration` rule cannot be
  read at the pin → taken as true (delta A15, E-01). **Not ported:** NourishmentAlwaysEatMixin — `can_eat` gets no
  player at the pin (engine gap **E-18**, delta A16). Note: `horizon-ctl` sprint does not sprint (4.2 b/s) and the
  client's food/saturation in `state` can be stale — use `/data get entity` for food tests.
- M4-04 Comfort: +2.0 HP over 170 ticks with saturation 0 (rule every 80 ticks of remaining duration); none while
  saturation > 0; Regeneration check in code (unit).
- M4-05: mushroom stew stacks to 16; rabbit stew 3 → 17 food (14/0.75), Jump Boost II 200, Nourishment 6000, bowl back.
  Straw bed → "Hay Bed" **not applied**: the engine ignores an `item_name` default-component override (an explicit
  `item_name` patch does display "Hay Bed") → engine gap **E-19**, delta A17. Config toggles wait for M9-01.
- M4-06: tamed wolf healed 4 → 40 + Speed/Strength/Resistance 6000, eat sound, bowl returned; tamed horse 5 → 53 + Speed II
  + Jump Boost 6000, feed consumed. A wild wolf passes to vanilla, which takes dog food as breeding meat (#minecraft:meat)
  and sets InLove — same as FD (its handler passes for untamed). Star particles sent server-side (`send_particles`).
- M4-07: straw/canvas/tree_bark burn 100/400/200 ticks in a furnace (measured `lit_total_time`); tree bark in a composter:
  42 rises in 172 counted insertions at level > 0 (0.244; 99% band for p = 0.3 at n = 172: 36–67) → PASS.
- M4-08: player Nourishment/Comfort and the wolf's/horse's effects keep their remaining time across save & quit and a full
  restart (only the load-to-freeze gap, 15–28 ticks); on the dedicated server a nourished guest (Bob) kept food 20 / sat 20
  under Hunger 251 for 10 s, while 2 s without Nourishment took saturation to 8.
- not done here: FD's food-effect tooltips (ConsumableItem tooltips) and the Nourishment hunger-bar overlay (client,
  carried to the HUD/tooltip work, M9); the recorded eating tour covers the first part only (later steps were run with
  horizon-ctl in the same world; results above).
- next: M5-01

## M5-01 Cabbage & onion — PASS — 2026-11-14
- build pin: afe471cf | mod commit: 8a5cc5f
- did: `cabbages`/`onions` declared on template `minecraft:wheat` (CropBlock age 0–7, growth, bonemeal, farmland
  survival as `ofFullCopy(WHEAT)`), outline = FD SHAPE_BY_AGE, no collision (`collision_shapes` empty, as wheat),
  `item = false` + `block_item` = the seed (getBaseSeedId). New item `cabbage_seeds` (template wheat_seeds, compost 0.3);
  `onion` keeps its food and gains `block = farmersdelight:onions`. Loot tables, models, tags
  (`villager_plantable_seeds`, `crops`, `maintains_farmland`) were already carried.
- evidence (FDTest, sky plot y=150 with water, `run/shots/m5-01/`):
  - planting by right-click on farmland: both crops placed at age 0; survival consumes one item each (seeds 20→19, onion 16→15).
  - bonemeal → age 7; mature render with FD textures (`mature.png`); random tick grew a cabbage 0→1 unaided.
  - survival harvest: 3 mature cabbages → 3 cabbage + 4 seeds; 2 mature onions → 6 onions; age-1 cabbage → 1 seed;
    age-0 onion → 1 onion (matches the carried loot tables incl. `minecraft:match_block` conditions).
  - onion used in the air is eaten (food 0→2 = FoodValues.ONION nutrition), no block placed.
  - pick-block (creative): cabbages → cabbage_seeds, onions → onion.
  - farmer villager (job site composter) with cabbage seeds + onions planted 9 cabbages, then 4 onions once the
    seeds ran out (onion 14→10); a vanilla wheat control farmer planted alongside (`villager_planted.png`).
- notes: crops cannot be placed by `horizon-ctl place` (needs a full solid face) — use look-at-block … up + `mouse right
  click`. Crops need light ≥8 or sky (vanilla canSurvive); the first try was underground and failed for vanilla wheat too.
  The test plot was found as plain dirt once (cause not identified: re-tilled farmland near water then held moisture 7
  under random_tick_speed 30; only the two cells under composters turned to dirt, as vanilla). Watch for it in M5-05/M5-08.
  FDTest difficulty is Peaceful (food tests need `/difficulty normal`); gamerules restored after the test.
- crop state persistence across save/restart is checked in the M5-08 demo.
- next: M5-02

## M5-02 Rope & rope fence — PASS (one engine gap) — 2026-11-14
- build pin: afe471cf | mod commit: 9bafcaf
- did: `rope` on template `minecraft:iron_bars` (RopeBlock extends IronBarsBlock: sides, waterlogging, outline and
  the horizontal side rule = vanilla attachsTo, which FD's tieToAnythingValid equals), + `tied_to_bell`, no collision,
  0.2 strength, wool sounds; `src/blocks/rope.rs`: placement on a top/bottom face ties only to rope/bars/panes/walls,
  `update_shape` sets tied_to_bell from the block above, empty-hand use rings a bell up the rope (≤24) via the bell's
  block event 1 + bell sound + game event, sneak use reels the lowest rope into the inventory (config default on),
  rope item on rope extends the column down (sneak: out of the clicked face), `is_pathfindable` true.
  `rope_fence` (template oak_fence, thin post/arms 16 high, collision 24 — 16 declared side tuples) and
  `rope_fence_gate` (template oak_fence_gate, FD shapes, FD open/close sounds, in_wall only between two walls);
  one `connects_to` part on `#minecraft:fences` = RopeFenceBlock#connectsTo + RopeFenceConnectionMixin; fire odds 0.
  Climbing comes from the carried `#minecraft:climbable` tag.
- evidence (FDTest sky rig, `run/shots/m5-02/`):
  - rope on a stone side face → `east=true`; on the bell's underside → `tied_to_bell=true`, no side ties.
  - rope on rope ×3 → column 154…151 (creative); survival extension 65 → 64 ropes.
  - empty-hand click on the rope → "Bell rings" subtitle (`bell.png`).
  - survival climb: hold jump in the rope, y 150 → 153.45; holding sneak keeps the player on it.
  - sneak + empty hand (survival): lowest rope (151) removed, ropes 64 → 65.
  - fence row (player-placed): oak|rope|rope|gate|rope|oak|oak → oak↛rope fence, rope fences joined, gate joined
    across its arm by both rope fences, rope fence↛oak, oak–oak joined; oak fence beside a rope fence gate stays
    unjoined (`fences.png`).
  - gate by redstone: "Rope Fence Gate unties"/"ties" subtitles (`redstone_close_sub.png`), `/playsound` resolves.
- engine gap **E-20**: opening the gate by hand plays no sound for the clicking player (vanilla's FenceGateBlock sends
  it to everyone but the clicker, whose client predicts it; Horizon's client prediction does not play the declared
  `open_sound`/`close_sound` — a vanilla oak gate creaks through the same path). Delta **A19**: rope placed against a
  side face directly under a bell shows `tied_to_bell=false` until the block above updates (a placement hook cannot read
  face sturdiness, so side-face placement is the template's own; tie-to-bell is visual — ringing searches up regardless).
  LAN check of the gate sound for other players: M5-08.
- next: M5-03

## M5-03 Tomatoes — PASS — 2026-11-14
- build pin: afe471cf | mod commit: 53c2022
- did: `budding_tomatoes` (template wheat, FD age 0-4 inside the template's 0-7, FD SHAPE_BY_AGE), `tomatoes`
  (template beetroots = CropBlock age 0-3, + `ropelogged`, always random-ticking), `tomatoes_on_rope` (beetroots,
  PUSH_PULL, picks tomato seeds); items `tomato_seeds` (places the budding bush; also the vine's item) and
  `rotten_tomato` (plain, stack 16; throwing is M8-06). `src/blocks/tomato.rs`: BuddingBush random growth (raw light ≥ 9,
  1 in 25/speed+1, FD's speed rule) and growth past max into `tomatoes`; budding bonemeal +1..4 carrying into the vine;
  vine growth with vanilla CropBlock.getGrowthSpeed (`#minecraft:grows_crops`, as 26.3), rope climbing (tagged
  `#farmersdelight:ropes`, ≤ 3 hanging), FD's bonemeal validity/recursion, harvest on use (1-2 + 5% rotten, pick sound,
  age 0), bone meal passes only on an unripe vine, survival above another vine by light only, mayPlaceOn
  `#minecraft:supports_crops`, rope permanence (`remove_block` for the hanging vine and legacy ropelogged vines),
  legacy ropelogged → hanging conversion. `rope_state_for_vine` = RopeBlock.getStateWithConnections(UP).
- evidence (FDTest farmland plot y=149):
  - seeds → `budding_tomatoes[age=0]` ×3; bone meal: budding age 2 → `tomatoes[age=2]` (carry-over) → 3.
  - bone meal on a ripe vine harvests it (as FD: useItemOn passes only for unripe vines) — 5 tomatoes on the ground.
  - with rope ×4 above: bone meal climbed one (`tomatoes_on_rope[age=3]` at 151); random_tick_speed 200 for 20 s: vine
    151-153 hanging, rope 154 left (≤ 3 cap); the other two buds grew into vines unaided.
  - empty-hand click on a ripe hanging vine → 2 tomatoes, age 0. Breaking a hanging vine (survival) → rope left;
    the unsupported vine above it → rope. Breaking the ripe base → tomato 2 + seeds 1 + rotten 1 (first break's
    reported drops), the hanging vine above → rope.
- delta **A20**: a ripe hanging vine knocked down by losing its support turns into rope without dropping its 1-2
  tomatoes (the shape update cannot destroy with drops; FD ticks and `destroyBlock`s it). Breaking it drops normally.
  Ravager trampling comes from the crop templates' own entityInside.
- next: M5-04

## M5-04 Rice — PASS — 2026-11-14
- build pin: afe471cf | mod commit: cc7f18d
- did: `rice` on template `minecraft:kelp` (a plant that always holds a water source = RiceBlock#getFluidState,
  takes no other liquid, not replaceable; its age 0-25 carries FD's 0-3) + `supporting`; `rice_panicles` on
  beetroots (CropBlock age 0-3). Items `rice` (places rice, villager food 2, compost 0.3) and `rice_panicle`
  (compost 0.3). `src/blocks/rice.rs`: placement only into a full water source at age 0; growth (light ≥ 6 above,
  1 in 3; panicles appear above at max age), survival on `#minecraft:supports_vegetation` (VegetationBlock in 26.3)
  or `#minecraft:supports_crops`, shape update (air when unsupported, water tick, `supporting` from above), bonemeal
  (+1..4, carry into new panicles or the panicles' own meal); panicles grow as a plain CropBlock (vanilla speed, not
  beetroots' 1-in-3), only on rice, CropBlock bone meal /3; RiceItem's "must be planted in shallow water" message.
  Compost test now skips block rows with `item = false` (their item is its own items row).
- evidence (FDTest paddy: dirt under a 3×3 water pool):
  - rice ×3 placed from above into the water → `rice[age=0,supporting=false]`; on dry dirt: nothing placed, action
    bar "This seed must be planted in shallow water."
  - bone meal: age 1 → 2 → 3 + `rice_panicles[age=0]` above (`supporting=true`) → panicles 1 → 2 → 3.
  - random_tick_speed 100, 25 s: the other two rice plants reached age 3 and grew panicles (`run/shots/m5-04/paddy.png`:
    FD models in water).
  - survival: ripe panicles → 1 rice panicle (base `supporting=false`); base → 1 rice, cell back to a water source.
  - knife harvest of panicles (→ rice) is in the carried loot table (match_tool `#farmersdelight:tools/knives`); checked
    with the knives in M8.
- next: M5-05

## M5-05 Rich soil & farmland — IN PROGRESS (declarations only) — 2026-11-14
- build pin: afe471cf | mod commit: 2965322
- done: `rich_soil` (template dirt, random_ticks) and `rich_soil_farmland` (template farmland, base_block rich_soil)
  declared; not built or run yet.
- design decided (to implement in `src/blocks/rich_soil.rs`):
  - rich soil `random_tick` (Cancel): brown/red mushroom above → `*_mushroom_colony` (needs M5-07 blocks); else
    level random nextFloat ≤ 0.2 → queue a boost. Random ticks only get a LevelWriteHost, so boosts go to a shared
    queue drained in `Hooks::server_tick`: `level.bone_meal(host, pos)` on the plant above (skip
    `#farmersdelight:planted_from_below` / `#farmersdelight:unaffected_by_rich_soil`), else below if planted_from_below.
  - `level_bone_meal` = dispenser path: emits level event 1505 (15 particles), which is exactly FD's
    RichSoilBoostParticlesPayload handler (BoneMealItem.addGrowthParticles(pos, 15)) → **no custom payload** needed,
    others see it. Delta **A21** (to record): the success roll applies (FD calls performBonemeal directly), boost lands
    one tick later, growWaterPlant fallback in water.
  - farmland `random_tick` (Cancel): water in box ±4, y 0..1 or rain above → moisture 7 (flag 2); else −1 if > 0, never
    reverts to soil; at 7: skip unaffected tag / tall flowers (sunflower, lilac, rose_bush, peony), separate RNG ≤ 0.2 → queue.
  - farmland `can_survive`: melon/pumpkin above → Some(true), else None. `fall_on` part no-op (FD fallOn = damage only;
    verify in game that this suppresses trampling vs vanilla farmland; `farmland_trample` gets only a raw u32 state id).
  - rich soil `tool_modified_state`: HoeTill with Some(final) → rich_soil_farmland; ShovelFlatten → Cancel (no path).
  - growth-speed probe: Horizon's CropBlock.getGrowthSpeed uses `#minecraft:grows_crops` (horizon-world
    level/tick.rs crop_growth_speed), which the carried tag extends with rich_soil_farmland → expected PASS; confirm in game.
- next: write rich_soil.rs, wire register + server_tick drain, build, test (tilling, moisture, boost, trample, probe).
