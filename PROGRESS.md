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

## M5-05 Rich soil & farmland — PASS — 2026-11-15
- build pin: afe471cf | mod commit: 791a7da
- did: `src/blocks/rich_soil.rs`. Rich soil: random tick (mushroom above → colony once M5-07 declares it, until then
  the mushroom is left alone; else 0.2 → boost the plant above, or the `#planted_from_below` one below, skipping
  `#unaffected_by_rich_soil`); hoe → `rich_soil_farmland` (the dirt template gives no answer for a mod block, so the
  part applies HoeItem.onlyIfAirAbove itself); shovel makes no path. Farmland: FD's random tick (water within 4 at y
  0..1 or rain → 7, else −1, never back to soil; at 7 a 0.2 boost with FD's own random, tall flowers skipped),
  canSurvive under melon/pumpkin, no trampling (Q11). Boosts are queued in the random tick and applied at the end of
  the same server tick with `LevelRef::bone_meal` (level event 1505 = the payload's 15 particles); the
  `rich_soil_boost_particles` payload is not registered. Config value `richSoilBoostChance` 0.2 (constant until M9-01).
- evidence (FDTest rig at 18..34, 160, 18..26; `run/shots/m5-05/`):
  - shovel on rich soil: unchanged; hoe with stone above: unchanged; hoe with air above → `rich_soil_farmland[moisture=0]`.
  - farmland row x=20..24 with water at x=25, random_tick_speed 300 for 12 s: x=21..24 → moisture 7, x=20 (5 away) stays
    0; wheat on the moist ones age 0 → 6-7; a moisture-7 farmland with no water dried to 0 and stayed farmland.
  - boosts: oak sapling on rich soil → tree; cave vines under rich soil → `berries=true`; short grass (unaffected tag)
    unchanged; brown mushroom unchanged (colony block comes in M5-07).
  - trampling: before the cancel, a 6-block survival fall turned the farmland to rich soil and popped the wheat
    (vanilla farmland → dirt the same way); with it, three falls leave `rich_soil_farmland[moisture=7]` and
    `wheat[age=3]`, the first landing after a restart included; fall damage applies (20 → 15, stone the same).
- growth-speed probe (U-13): PASS at source level + in game — Horizon's crop growth speed counts
  `#minecraft:grows_crops` blocks and their `moisture` (horizon-world level/tick.rs), the carried tag adds
  rich_soil_farmland, wheat grew on it. Speed alone was not measured apart from the boost. E-04 not needed.
- deltas/probes: **A21** boost goes through the dispenser's bone meal path: the block's success roll applies (FD calls
  performBonemeal directly), it lands at the end of the tick, and a plant in water can take growWaterPlant. Particles
  seen by a second client: M5-08. Engine gap **E-22**: `farmland_trample` is told only the game's numeric state and has
  no host, and a farmland-template block cannot opt out of trampling; the mod computes the block's state numbers at
  server start (sum of earlier blocks' state counts — correct at this pin) and re-learns them from the `fall_on` that
  follows if they differ (one trample would slip through then).
- also fixed (user report, M5-04): planted rice showed no water. The client takes a block's water from vanilla's name
  list (kelp, seagrass, …) or its `waterlogged` property, so a kelp-template mod block holds water on the server but
  draws none (engine gap **E-21**). Rice now declares `waterlogged` (always true); existing rice loads with it. Delta
  **A22**: rice has a `waterlogged` state property FD's does not. Confirmed by the user in game.
- next: M5-06

## M5-06 Organic compost — PASS — 2026-11-15
- build pin: afe471cf | mod commit: 9f4b88c
- did: `organic_compost` on the dirt template (hardness 1.2, crop sounds, random ticks; `composting` 0-7 declared as an
  enum of the digits — declared properties are boolean or enum). `src/blocks/compost.rs`: FD's random tick (0.02 per
  `#compost_activators` in the 3×3×3, +0.1/0.05 by sky light above the neighbours, +0.1 with water; at 7 → rich soil,
  flag 3) and comparator signal 8 − stage. Unit test of the chance terms.
- evidence (FDTest):
  - statistical: a 16×16 layer under open sky, random_tick_speed 100 for 303 game ticks (`time query gametime`
    82477 → 82780, ~298 ticking): stages 0..7 = 31/79/58/58/17/11/1/1 → 505 advances; expected
    (196×0.28 + 56×0.22 + 4×0.18) × 298×100/4096 = 494 (+2.2%, within ±10%).
  - comparator → redstone wire: stage 0/1/3/6/7 → power 8/7/5/2/1; stage 7 then turned into `rich_soil` (power 0).
- deltas: **A5** mycelium motes not drawn (E-06), as planned.
- next: M5-07

## M5-07 Mushroom colonies — PASS — 2026-11-15
- build pin: afe471cf | mod commit: 9f4b88c
- did: `brown_mushroom_colony`/`red_mushroom_colony` on the mushroom templates (`age` 0-3 enum, FD shapes, pick =
  the mushroom, compostable 1.0). `src/blocks/colony.rs`: growth one in 4 on `#grows_mushroom_colonies` with no light
  check (Q9) and no template spreading; survival = the template's MushroomBlock rule plus any light on that tag; bone
  meal +1..2; shears (`c:tools/shear`) −1 stage and one mushroom; knife (`#farmersdelight:tools/knives`) all stages;
  both cost 1 durability and send block particles; the item places at age 3. Rich soil's mushroom → colony turn is
  live now.
- evidence (FDTest, `run/shots/m5-05/colonies.png`): brown and red mushrooms on rich soil in daylight (block light 1,
  sky 15) → colonies, age 3 within 12 s at random_tick_speed 300. Survival: shears on age 3 → age 2, +1 brown
  mushroom, shears 238 → 237; flint knife on age 3 → age 0, 3 red mushrooms dropped, knife −1; bone meal 0 → 2 → 3;
  colony item on rich soil → `[age=3]`, on stone in daylight → not placed; breaking age 3 and age 2 → 5 + 4 mushrooms
  (carried loot table).
- next: M5-08

## M5-08 M5 demo — PASS — 2026-11-15
- build pin: afe471cf | mod commit: 9f4b88c
- single player (FDTest demo farm at 18..26, 160..164, 18 plus the M5-05..07 rigs; `run/shots/m5-08/farm_*.png`):
  cabbages[age=4], onions[age=2], budding_tomatoes[age=2], tomatoes[age=3], tomatoes_on_rope[age=1] under rope,
  rice[age=3,supporting=true] + rice_panicles[age=2], open rope fence gate, organic_compost[composting=4],
  colonies age 2/3, rich_soil_farmland moisture 5/7 with wheat — 15 block states read before `quit` and after a
  fresh launch: identical. Per-feature behaviour is in the M5-01…M5-07 entries (each run in game).
- LAN (dedicated server re-exported with this build, clients Alice and Bob refreshed from it):
  - Alice opens the rope fence gate by hand → Bob's subtitles show "Rope Fence Gate unties"
    (`bob_gate_open.png`): other players hear it; only the clicker's own client is silent (E-20).
  - wheat on moist rich soil farmland, random_tick_speed 60: Bob sees the green growth particles on the boosted
    wheat and the "Bone Meal crinkles" subtitle (`bob_boost_3.png`).
- deltas: A21 addition — the boost from rich **soil** also plays the bone meal sound (FD's payload draws particles
  only; the farmland's boost already uses level event 1505 in FD, sound included).
- not covered here: compost → rich soil → tilled → crop as one continuous survival run (each step was run on its own).
- next: M6-01

## M6-01 Wild crops — IN PROGRESS (code written, NOT run in game) — 2026-11-15
- build pin: afe471cf | mod commit: 1cfa305
- done (builds, unit tests pass, decl tests pass; nothing verified in game):
  - decls: 6 `wild_*` + `sandy_shrub` on template `minecraft:short_grass` (not in `#minecraft:replaceable`, burns 60/100
    as FD registers), `wild_rice` on `minecraft:tall_grass` with a declared `waterlogged` (default dry lower half;
    outline stays tall grass's, FD's is a full cube); compostable 0.65 (shrub 0.3).
  - `src/blocks/wild.rs`: ground rules (`#grows_wild_crops` / `#minecraft:sand`), wild crop bone meal (80%, ≤10 in
    9×3×9, FD's random walk), shrub not bonemealable, wild rice placement only into a full water block with air
    above, canSurvive per half, bone meal 30% → pops one wild rice (queued, dropped at end of tick: a level write
    cannot spawn items).
  - stew effect: the game's flower→stew table is fixed to vanilla names (horizon-items flowers.rs), so
    `entity_interact` handles a wild crop used on a **brown mooshroom** (reads the cow's NBT `Type`, merges
    `stew_effects` [{id, duration = seconds×20}], eats one, eat sound; two smoke puffs if it already holds effects).
    Vanilla's bowl path then gives the stew. 26.3 has no crafting route for FD's flowers (FD ships no stew recipes).
- to test in game: place each on its ground / wrong ground; bone meal spread and the 10 cap; shrub on sand only;
  wild rice in water (both halves, lower `waterlogged=true`, breaking either half, water left behind — E-13 may
  apply); bone meal on wild rice; `/summon mooshroom ~ ~ ~ {Type:"brown"}`, feed each crop, bowl → stew, eat → effect;
  loot (carried tables); whether `may_place_on`/`can_survive` parts are honoured for the short_grass template.
- M6-02 findings so far (nothing written yet) — the port can be close to exact, better than the planned degraded form:
  - `farmersdelight:in_order` == vanilla 26.3 `minecraft:overlay` (places every inline placed feature, true if any;
    horizon-world worldgen/feature_graph.rs `Self::Overlay`). `minecraft:sequence` stops at the first failure — not it.
  - `farmersdelight:biome_tag` (always `#minecraft:is_overworld`) → drop the placement entry and put
    `#minecraft:is_overworld` into the biome modifier's `neoforge:and`.
  - `farmersdelight:wild_rice` feature → `minecraft:simple_block` of `wild_rice[half=lower,waterlogged=true]`
    (the placed feature already filters water / air above / substrate below; FD's extra "full source" check is lost).
    Unknown: whether simple_block places the upper half for a tall_grass-template mod block, and whether worldgen
    (worker threads, no mod parts) applies the **template's** canSurvive — short_grass on sand would then refuse wild
    cabbages/beetroots/shrubs. Test with `/place feature` first.
  - biome selection: `data/farmersdelight/neoforge/biome_modifier/*.json`, `neoforge:add_features`, step
    `vegetal_decoration`, biomes = `neoforge:and` [#allow tag, `neoforge:not` #deny tag, `horizon:climate`
    temperature {min,max}] (docs/modding content-mods.md "biome modifiers"; climate = base temperature, as
    FDBiomeSelector). Ranges: carrots/onions 0.4–0.9, potatoes 0.1–0.3, rice/tomatoes −4..4 (= none), colonies /
    cabbage / beetroots tag only.
  - FD's biome tags reference Fabric convention biome tags the mod must ship (same generator as the item tags, new
    table `tools/conventional/biome_tags.toml.in`, output `data/c/tags/worldgen/biome/`). Values read from the
    user's local fabric-convention-tags-v2 4.10.4 jar (Prism 26.3 instance): is_mushroom = [mushroom_fields];
    is_beach = [#minecraft:is_beach (optional)]; is_underground = [#c:is_cave (optional)]; is_cave = [lush_caves,
    dripstone_caves, sulfur_caves, deep_dark]; is_wet/overworld = [swamp, mangrove_swamp, jungle, bamboo_jungle,
    sparse_jungle, beach, lush_caves, dripstone_caves, sulfur_caves]; is_hot/overworld = [jungle, bamboo_jungle,
    sparse_jungle, desert, badlands, wooded_badlands, eroded_badlands, savanna, savanna_plateau, windswept_savanna,
    stony_peaks, mushroom_fields, warm_ocean].
  - carry script needs two new scripted rules (worldgen type rename, placement-entry drop) + one override
    (wild_rice feature); record them in `tools/carry_rules.toml.in` and the manifest; `tools/test_carry.py` must cover them.
- next: test M6-01 in game, then M6-02 as above, M6-03 (village pools/processors, `horizon:add_to`), M6-04 (trades,
  farmer POI probe), M6-05 (fresh-world demo).

## Creative tab (user request) — NOT RUN in game — 2026-11-15
- did: `tools/gen_tab_decl.py --fd <FD checkout>` writes `tools/decl/tabs.toml.in` (FD's tab: title
  `itemGroup.farmersdelight`, icon the stove, 188 items in ModItems' register*WithTab order, dyed canvas signs
  resolved); `tools/gen_decls.py` renders `data/farmersdelight/horizon/creative_tab/farmersdelight.json`. Items not
  declared yet are listed too (Horizon leaves out items the game doesn't have).
- to do: `run/rebuild.sh`, open the creative inventory, confirm the "Farmer's Delight" tab, its icon and order; check
  the load log for refusals (if undeclared ids are refused rather than skipped, filter the list to declared items in
  gen_tab_decl.py); add gen_tab_decl to CI drift checks.
- next: test M6-01 in game (previous entry), then M6-02…M6-05.

## M6-01 Wild crops — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- fixed while testing:
  - wild rice broke right after placement and took its water: FD's lower half answers a water source itself
    (getFluidState); the engine reads the tall_grass template's fluid (none), so the canSurvive water check failed.
    canSurvive now counts a placed lower half as water; `remove_block` leaves water for the lower half (E-13).
  - wild rice dropped rice when broken in creative (and from both halves in survival). Engine gap **E-23**: loot
    `minecraft:location_check` reads only `predicate.biomes`; `offsetX/Y/Z` and the `block` predicate are ignored, so
    FD's "other half must stand" pool conditions always pass. Workaround in `block_drops` (fires on every drop path;
    `loot_drops` does not see neighbour pops): a wild rice half popped without a breaker whose partner is gone drops
    nothing. Delta **A23**: a player's break always drops from the broken half (FD: only while the partner stands — a
    lone half cannot occur through play); explosions taking both halves are not reproduced exactly. FD's
    `advancement/main/harvest_ropelogged_tomato.json` also uses `location_check` (owner: its advancement task).
- evidence (FDTest rig x=46..64, y=158..162, z=10..56; screenshots `run/shots/m6-01/`):
  - placement: 6 wild crops placed on grass, dirt and sand, refused on stone; sandy shrub on sand only (refused on
    grass, dirt, stone); sand replaced by stone under a wild cabbage → it popped (canSurvive).
  - bone meal: on a 19×19 grass field one wild cabbage + 65 bone meals → exactly 10 cabbages in the 9×3×9 (cap holds;
    first 5 uses gave +4, 80%); no tall grass appears (template's bone meal not used); sandy shrub: no change.
  - wild rice: into a water source → `[half=lower,waterlogged=true]` + `[half=upper,waterlogged=false]`; dry spot
    refused; breaking either half in creative and survival leaves `minecraft:water[level=0]`; drops creative 0/0,
    survival 1 rice / 1 rice (FD: same). Bone meal ×40 on wild rice → 12 wild rice popped (30%), plant kept.
  - brown mooshroom (survival): each crop eaten (stack −1), a second feed refused (stack kept); bowl → suspicious stew
    with strength 120, fire_resistance 120, poison 200, mining_fatigue 120, nausea (160 default), water_breathing
    (160 default) — FD's seconds×20. Eating the water-breathing stew → water_breathing 153 ticks left a moment later.
  - loot (survival, bare hand): wild cabbages → cabbage_seeds; sandy shrub → nothing (12.5% beetroot seeds).
    Shears give no wild crop item until `farmersdelight:can_item_perform_ability` exists (M8-05).
  - creative tab: second tab page shows "Farmer's Delight" with the stove icon, FD's order (stove, cooking pot,
    skillet, cutting board, …); no load warnings for the tab (creative-tabs-p2.png). Closes the creative-tab entry.
- not covered: fire spread/burn odds of the wild crops (declared 60/100, not run).
- next: M6-02 (design in the M6-01 IN PROGRESS entry above).

## M6-02 Worldgen — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did (near-exact instead of the planned degraded form):
  - carry: two scripted `[[rewrite]]` rules (`tools/carry_rules.toml.in`, manifest rule "rewrite", covered by
    `tools/test_carry.py`): `farmersdelight:in_order` → vanilla `minecraft:overlay` (identical semantics), and the
    `farmersdelight:biome_tag` (#is_overworld) placement entry dropped (always passes in the overworld, draws no
    random). Override `worldgen/feature/wild_rice.json`: overlay of `simple_block` lower half (waterlogged) at the
    origin + `simple_block` upper half one up.
  - biome selection: FD's `ModBiomeModifiers` (Fabric `BiomeModifications`) restated as 9 NeoForge `add_features`
    modifiers in `data/farmersdelight/neoforge/biome_modifier/NN_*.json`, generated from
    `tools/worldgen/biome_modifiers.toml.in` by `tools/gen_biome_modifiers.py` (`--check`): `neoforge:and` of the
    allow tag, `neoforge:not` deny tag, `horizon:climate` base temperature (carrots/onions 0.4–0.9, potatoes 0.1–0.3).
    File prefix NN keeps FD's registration order (Fabric sorts by phase, then registration — checked in the user's
    fabric-biome-api-v1 jar with javap), so each step's feature order matches.
  - Fabric convention biome tags FD's tags reference: `tools/conventional/biome_tags.toml.in` →
    `data/c/tags/worldgen/biome/` (is_mushroom, is_beach, is_underground, is_cave, is_wet/overworld,
    is_hot/overworld), same generator as the item tags.
  - wild_cabbages, wild_beetroots, sandy_shrub: template `minecraft:short_dry_grass` (was short_grass), grass sounds,
    plant map colour.
- engine gaps found (mod-side workarounds in place):
  - **E-24** worldgen `simple_block` places both halves only for vanilla's double plants (a fixed list); a mod double
    plant gets its lower half alone. Workaround: the two-step overlay above.
  - **E-25** worldgen survival (`generator_decoration` `block_survives`) reads the template's static rule table
    (`survival-rules.json`), never mod parts. Short grass refuses sand, so beach cabbages/beetroots/shrubs never
    generated (0 on 11 beaches). Workaround: short_dry_grass template (`#supports_dry_vegetation` = sand, terracotta,
    `#supports_vegetation`); every FD feature's own filter already demands the right ground below.
- deltas: **A24** wild cabbages/beetroots/sandy shrub use the short dry grass template: the client predicts a
  placement on terracotta that the server refuses (snaps back, E-14), and on sand they may play the dry-vegetation
  ambient sound. **A25** generated wild rice is checked by the tall grass template's survival rule (E-25), so it can
  generate in flowing water where FD's canSurvive needs a source (the placed filter only asks for water). Placing by
  hand keeps FD's rule.
- evidence (new world `FDGen`, seed 0; `run/shots/m6-02/`):
  - load log: no worldgen/biome modifier warnings. Content cache biomes: every overworld biome got exactly the FD
    features FD's selectors give (e.g. plains/forest/river/oceans/deep_dark: carrots+onions; taiga, old growth
    pine/spruce taiga, windswept hills/forest/gravelly hills, stony shore: potatoes; desert/badlands/savanna/
    jungles/stony peaks/warm ocean/mushroom fields: tomatoes; swamp/mangrove/jungles/beach: rice; beach/snowy beach:
    cabbages+beetroots; mushroom fields: both colonies; carrots/onions not in mushroom fields or lush caves).
  - generated in new chunks (find-block, 64 radius): swamp −1175,414: 28 wild rice blocks (14 plants, lower
    waterlogged on dirt + dry upper; swamp-wild-rice.png); mushroom fields −503,−1154: 21 brown colonies; plains/
    forest: 9 carrots; desert 634,−1619: 11 tomatoes; taiga 2810,192: potatoes; beaches after the template change:
    cabbages (−6106,64,−6435, beach-wild-cabbages.png; 9122,−78; −129,−9137; 9064,−8985), beetroots (9102,−57: 32;
    −147,−9076), sandy shrubs. `/place feature` in FDTest: cabbages on sand, carrots (+coarse dirt, short grass),
    rice 30/30 with both halves.
  - restart: generated cabbages and both rice halves still there.
  - M6-01 behaviour re-checked with the new template: terracotta refused, grass accepted, bone meal spreads cabbages
    only.
- not covered: density against an FD reference run (no oracle: FD is never run here); red colony seen only in the
  modified-biome table, not found in the one mushroom field visited.
- next: M6-03 (village pools/processors, `horizon:add_to`).

## M6-03 Villages — IN PROGRESS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: `tools/worldgen/villages.toml.in` (FD's VillageStructures: 5 compost-pile houses with FD's weights, 3 crop
  rule sets on 5 farm processor lists) rendered by `tools/gen_worldgen.py` (renamed from gen_biome_modifiers.py; it
  renders the biome modifiers too, `--check`) into `horizon:add_to` files:
  `data/farmersdelight/worldgen/template_pool/village/<biome>_houses.json` (single_pool_element, processors
  minecraft:empty, rigid, weight 5/3/4/3/4) and `worldgen/processor_list/farm_<biome>.json` (one `minecraft:rule`
  processor appended; output states as 26.3 `{"id": …}` — the first try used `{"Name": …}`, which the engine refused
  silently and dropped the processor).
- evidence (world `FDGen`, seed 0, villages in fresh chunks):
  - content cache: each vanilla house pool ends with FD's piece, each farm list with FD's rule processor.
  - compost heaps generate: plains −848,25904 (6 organic compost) and −25312,−25264; snowy −384,−25280 (24);
    savanna 28352,2240 / −32368,−448 / −1552,−27392.
  - FD crops on farms: plains 24752,24864: 8 onions, 11 tomatoes, 56 wheat; plains −848,25904: 6 onions, 5 tomatoes;
    desert 31584,32960: 6 cabbages, 8 tomatoes, 25 wheat; desert −25344,−30272: 3/4/25. These match the rate expected
    when each RuleProcessor reseeds from the position (vanilla RuleProcessor: `RandomSource.create(Mth.getSeed(pos))`,
    engine `LegacyRandom::new(position_seed)`, `random_block_match` short-circuits like vanilla): FD's processor
    re-reads the draws vanilla's processor already used.
  - quirk (kept, not fixed): plains and taiga farms never get cabbages — vanilla's first rule there is also
    `wheat 0.3`, so FD's `wheat 0.3 → cabbages` sees the same first draw that vanilla already rejected. Same in FD on
    vanilla (order and seeding are vanilla's). Candidate quirk-table entry Q17.
- OPEN (blocks PASS): savanna farms get almost no FD crops. A `/place structure minecraft:village_savanna` at
  40512,70,40512 → farmland tops: 228 wheat, 62 melon stems, 0 FD crops, 68 water, 4 air (expected ≈ 20% cabbages,
  21% tomatoes); natural savanna villages show a few (−1968,31632: 8 cabbages, 7 tomatoes, 303 wheat). Vanilla's own
  melon rate there is 21% instead of 10%, as if those farms ran another list. `farm_savanna` in the content cache
  holds both processors; the savanna farm pieces reference `minecraft:farm_savanna`; nothing savanna-specific in
  `horizon-world/src/worldgen/structure`. Next: an engine-side probe (structure placement of
  `village/savanna/houses/savanna_small_farm` with `farm_savanna`, printing the processors applied per block).
- engine gap **E-26**: worldgen data (template pools, processor lists, biome modifiers) has no load conditions and
  there is no code route to add pool elements or processors, so FD's `generateVillageCompostHeaps` and
  `generateFDCropsOnVillageFarms` (default true) cannot switch the additions off. Delta **A26**: both keys have no
  effect; villages always get FD's additions (FD's default).
- not covered: trades (M6-04), taiga/snowy farm rates measured on few villages.
- next: M6-03 savanna probe, then M6-04.

## M6-03 Villages — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit) | no mod code change since 936a5da
- did: closed the open savanna question from the IN PROGRESS entry above. Not an engine problem: savanna village
  **street** pieces (`streets/crossroad_07`, `straight_06`, `straight_08`, `straight_11`) carry their own wheat and
  melon-stem plots in the structure NBT (153 wheat, 31 melon stems ≈ 20% melon) and use the street processor list,
  not `farm_savanna`. The `/place structure` village at 40512,70,40512 had only street plots, which explains its 21%
  melon and 0 FD crops; natural savanna villages mix street plots with real farms (hence the low FD share there).
  FD on vanilla leaves street plots alone too.
- evidence (world FDTest, far from the user's area; `run/shots/m6-03/savanna-jigsaw-counts.txt`):
  - 37 × `/place jigsaw minecraft:village/savanna/houses minecraft:building_entrance 1 X 200 Z` around 50000,50000:
    farm pieces with `farm_savanna` gave wheat 208, cabbages 95, tomatoes 91, melon stems 35 → 48.5 / 22.1 / 21.2 /
    8.2% vs expected 49 / 20 / 21 / 10% (vanilla melon 0.1, then FD's processor reseeded per position). FD compost
    piece also placed (14 composters, 25 organic compost, 5 rich soil farmland).
  - with the earlier entry: compost heaps in plains/snowy/savanna villages, FD crops on plains/desert/savanna farms.
- quirk added to PORT_PLAN §4.1 as **Q19** (Q17 was already taken): plains/taiga farms never get cabbages
  (position-seeded draw shared with vanilla's carrot rule). Kept.
- deltas: A26 (config keys inert, E-26) stands; "config off → none" in the acceptance cannot be met (recorded delta).
- not covered: a screenshot (the player landed in ocean at night; counts are the evidence); taiga/snowy rates on
  more villages.
- next: M6-04 (trades & farmer POI).

## M6-04 Trades & farmer POI — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: the 8 carried trades (`data/farmersdelight/villager_trade/**`) and the 3 `minecraft:villager_trade/*` tags
  were already shipped, but no FD trade was ever offered: 30 level-1 farmers sampled, 0 FD trades (P ≈ 10⁻⁹ if
  the tag held 7 entries). Cause: the engine's villager_trade tag reader keeps only plain string entries
  (`horizon-client/src/merchant.rs` `Trades::load`: `filter_map(|e| e.as_str())`), and FD writes its entries as
  `{"id": …, "required": false}`. Engine gap **E-27**. Workaround: new carry rewrite `flatten_tag_entries`
  (`tools/carry_fd.py`, rule in `tools/carry_rules.toml.in`, unit test in `tools/test_carry.py`) writes those entries
  as plain ids in the 3 tags. Delta **A27**: the entries are no longer optional; no player-visible difference,
  because every listed trade ships in this mod (vanilla would only differ if a trade file were missing).
- farmer POI probe (U-14, E-10): no engine change needed. The engine's farmer secondary-POI scan and
  HarvestFarmland use any block of class `FarmlandBlock` (`mobs/villager_secondary.rs` `sense_secondary`,
  `villager_brain.rs` `farmland`), which is what FD's `VillagersTargetRichSoilMixin` adds; rich soil farmland
  is declared on the `minecraft:farmland` template. **E-10 not needed.**
- evidence (FDTest, sky platform at 50000,220,50000, far from the user's spot; `run/shots/m6-04/`):
  - after the fix, `/summon` farmer (NoAI) + right-click + `/data get entity … Offers`
    (`trade_sample.py`): L1 12 villagers → 5 × 26 tomato→emerald, 3 × 26 onion→emerald; L2 12 → 7 × 16 cabbage,
    3 × 20 rice; wandering trader 10 → tomato seeds ×2, rice, onion for 1 emerald. Counts match the carried JSON.
    Before the fix: `before-fix-farmer-l1.txt` (30 villagers, vanilla trades only).
  - a farmer (AI on, composter job site) on a 9×5 rich soil farmland plot with water, 18 ripe wheat + 26 bare
    cells, `time set 2500`: within 90 s it harvested ripe wheat (replanted at age 0) and sowed bare cells
    (18 → 24 wheat); `farmer-rich-soil.png`.
- not covered: completing a trade through the merchant screen (engine path, data only here); trades after a
  restart (M6-05).
- world changes: time of day set to 2500; test platform, plot and the M6-03 jigsaw pieces stay at ~50000,50000.
- next: M6-05 (M6 demo).

## M6-05 M6 demo — PASS — 2026-11-15
- build pin: afe471cf | mod commit: aee07de (no code change in this task) | world `FDM6` (new, seed 6, creative)
- evidence (`run/shots/m6-05/`, `latest.log` copied there):
  - wild crops in fresh chunks (find-block, radius 64): wild onions 76,76,629 and 139,82,687 (forest/beach edge);
    wild carrots −14,80,1114 and 286,79,1607; wild beetroots + sandy shrubs on beaches 273,65,1660 (5 + 8),
    100,63,1212, −42,64,1421 (`wild-beetroots-shrub.png`).
  - plains village (`/locate structure` → 16,~,1360): FD compost heap −2…2,73–74,1368–1371 (6 organic compost,
    1 rich soil; `village-compost-heap.png`); FD crops on both farms: 7,73,1320 (onions 4, tomatoes 4 beside
    wheat/carrots/potatoes; `village-farm-fd-crops.png`) and 47,64,1361 (onions, tomatoes).
  - trades: farmers summoned in the village, offers generated by right-click: L1 = 20 wheat / **26 tomatoes** →
    emerald (`farmer-l1-trades-before-restart.png`), L2 = **16 cabbages** → emerald / emerald → 4 pumpkin pie.
  - full restart (`ctl quit`, `launch.sh FDM6`): both farmers keep the same offers (`/data get … Offers`); trade
    screen still lists the tomato trade (`farmer-l1-trade-after-restart.png`); trade completed in survival:
    26 tomatoes consumed, 1 emerald received, offer `uses: 1`, villager Xp 2.
  - log: no worldgen/biome-modifier/trade warnings except `farmersdelight:rice has no block survival rule: unknown
    growing plant family` (worldgen loader; the rice crop is never placed by worldgen, wild rice is — harmless,
    noted) and the known damage-tag warnings (E-15).
- not covered: LAN second client (M6 adds no block entity, item state or effect; exit rule does not require it);
  village config off (A26, E-26); a trade first made in creative left the 26 tomatoes visible in the creative
  inventory until the gamemode change (server consumed them; engine creative inventory, not FD-specific, not
  investigated).
- **M6 closed.**
- next: M7-01 (PORT_PLAN §11).

## M7-01 Feasts — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: 6 feast blocks declared (`tools/decl/blocks.toml.in`, stonecutter template for `facing` opposite the
  placer, declared `servings` 0–4 / medley 0–8, placed full; outlines generated from BlockShapes/feast classes
  incl. ShapeUtils rotation; gleaming salad `state_light` = servings × 3; push reaction POPPED; items stack to 1;
  stuffed pumpkin block compostable 1.0, caught by `tools/test_compost_fuel_decls.py`). Code `src/blocks/feast.rs`:
  FeastBlock#takeServing (container rule, creative keeps the bowl, leftovers break with loot on the next use,
  stuffed pumpkin vanishes with its last serving, armor-equip sound, 3 block particles), medley serving order,
  survival/updateShape, comparator = servings, not pathfindable, `use_without_item` Pass (template menu never
  opens), PlaceableItem "Placeable" tooltip, gleaming salad sparkles.
- evidence (FDTest, sky platform 50096–50108,220,49996–50008; `run/shots/m7-01/`):
  - roast chicken in survival with 6 bowls: servings 4→0, one bowl used and one roast chicken given per use;
    next use broke the leftovers, dropping bowl + bone meal (carried loot table). `feasts-placed.png`.
  - empty hand on the ham: action bar "You need a Bowl to eat this.", no stonecutter menu (`need-bowl.png`).
  - rice roll medley, empty hand: 3 kelp roll slices, 3 salmon rolls, 2 cod rolls in that order, then leftovers
    broke. Stuffed pumpkin: 4 servings, block gone after the 4th, nothing dropped (its table drops only at 4).
  - breaking a full ham in survival drops the ham block; placed from the item looking west → `facing=east`.
  - comparator behind the shepherd's pie: power 4, 3 after a serving (and 2→1 after the restart).
  - gleaming salad block light 12 at 4 servings, 6 at 2. Sparkles: 66 in 30 s from 21 salads (expected ≈ 56–63
    from vanilla's animateTick odds × 0.05 × servings); one visible in `salad-sparkle-night.png`.
  - support removed → the feast dropped itself; enclosed water source beside a feast does not flow into it
    (forceSolidOn behaviour holds); piston push pops it with its drop.
  - creative with a bowl: served, bowl kept. Off hand: bone meal in the main hand + bowl in the off hand → action
    bar message for the main hand, then the off hand served (bowl 2→1, salad 3→2).
  - full restart (`ctl quit`, rebuild, `launch.sh FDTest`): feast states identical, serving still works.
- engine gap **E-28** (new): a remote client predicts a mod block's use from its template (`remote_block_reacts`
  in `game/network/remote/mod.rs`); the block Part's `use_item_on` is server-only, so the stonecutter's empty-hand
  use claimed every main-hand click and the client never sent the off hand's click. Workaround: `use_item_on_block`
  (Block phase, client side only) answers the feast's own prediction (Success when it serves or breaks, Pass when it
  needs a container), as FD's client-side `takeServing` does. Same gap likely for other template-backed blocks
  (cutting board, pot, skillet answer every main-hand click, so they are unaffected).
- deltas: **A28** feast survival uses the A9 approximation (the hook's level host has no face-sturdiness query; any
  non-air, non-replaceable block below supports it). **A29** gleaming salad sparkles come from a mod-side block
  entity that ticks on the client (no animate-tick hook for plain blocks, E-06); the salad therefore saves an empty
  `farmersdelight:gleaming_salad_block` block entity, which FD does not have. The rate per tick is vanilla's
  per-block animateTick chance (≈0.0229) × 0.05 × servings; vanilla's real odds vary with the player's distance.
- plan correction: PORT_PLAN M7-01 lists `destroy_overlay` false, but HideBlockBreakProgressMixin hides the crack
  only on the canvas rug (M7-07); feasts keep it.
- not covered: the "Placeable" tooltip (coded, not hovered in game: horizon-ctl has no hover), pathfinding (hook
  coded, not observed with a mob), LAN second client (M7-09).
- world changes: test platform with feasts, 20 gleaming salads, piston and comparator at ~50100,220,50000; time set
  to 1000; player back at 22,160,22 in creative. Test items left in the inventory: cod rolls 2, gleaming salads 2,
  kelp roll slices 3, roast chickens 4, salmon rolls 3, shepherd's pies 3, stuffed pumpkins 4, bowls 3 + 1 in the
  off hand, 1 extra bone meal (`clear` is not run on the user's inventory).
- next: M7-02 (pies & pumpkin pie).

## M7-02 Pies & pumpkin pie — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: 4 pie blocks declared (stonecutter template, declared `bites` 0–3, CAKE properties, outlines PieBlock.SHAPES
  rotated per facing, empty loot tables carried); apple/cheesecake/chocolate items compostable 1.0 + "Placeable"
  tooltip; hidden `farmersdelight:debug_pumpkin_pie` item is the pumpkin pie block's item (no tab). Code
  `src/blocks/pie.rs`: facing the way the placer looks (`state_for_placement`), eating a bite with any non-knife
  item in the main hand (canEat; FoodProperties#onConsume: eat sound, `feed(3, 0.3)`, burp; slice consume effect
  Speed via `apply_consume_effect`; eat sound + block particles at the pie; removed without drops after the 4th),
  knife (`c:tools/knife`, either hand) cuts a slice toward the player + ITEM_USED stat, comparator 4 − bites, not
  pathfindable; vanilla `minecraft:pumpkin_pie` ItemPart `use_on` places `farmersdelight:pumpkin_pie` (config
  `enablePumpkinPieSneakToPlace`, default false, in `src/config.rs`). Client predictions (E-28) for pie clicks and
  the pumpkin pie placement in `use_item_on_block`.
- support approximation refined (A28): feasts and pies now refuse, besides air and replaceables, floors known never
  to have a rigid top (fences, walls, carpets, beds, candles, buttons, pressure plates, flower pots, rails, bottom
  slabs, FD feasts/pies/cutting board/skillet/pot) — found when a pie went on top of a pie in game; refused now.
- evidence (FDTest, pad 50110–50120,219,49994–50006; `run/shots/m7-02/pies.png`):
  - apple pie placed in survival looking north → `facing=north`; full food: a click does nothing to the pie.
  - food drained (difficulty Easy for the test, Peaceful restored): 4 bites → food 0→3→6→9→12, saturation +1.8
    each, pie gone after the 4th, `speed` 600 ticks (icon and particles hidden, as FD's effect instance).
  - flint knife: 4 slices cut and picked up, comparator 4→3→2→1→0, knife undamaged.
  - vanilla pumpkin pie on the floor → `farmersdelight:pumpkin_pie[bites=0,facing=north]`, stack 3→2 on server
    and client, no eating started.
  - pie onto a pie: refused after the fix (server keeps the stack; the client shows one fewer until resync, the
    E-14 placement-prediction class).
- not covered: save/restart of pie bites (M7-09), "Placeable" tooltip (no hover in horizon-ctl), pick block of the
  pumpkin pie (gives the hidden item by declaration), sneak-to-place config on (inert constant, A26 style).
- incident: a teleport to the pad position before the pad was filled, then `gamemode survival`, killed the player
  (fall). keep_inventory is off: everything dropped; all items were collected back (the user's 4 tomatoes, 2 glow
  berries and bone meal included; the bone meal had fallen to y 68). The death message is in chat and the death
  in the statistics. Rigs are now built before teleporting, in creative.
- world changes: pad with 4 pies and a comparator at ~50115,220,50000; time 1000; player at 22,160,22 in creative.
  Test items added to the inventory since M7-01: apple pie slices 4, pumpkin pies 2.
- next: M7-03 (storage blocks).

## M7-03 Storage blocks — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: 9 blocks declared: 6 crates (oak-planks template, strength 2.0/3.0), rice bale (hay template + declared 6-way
  `facing`, compostable 0.85), rice bag (white-wool template), straw bale (hay template). Code
  `src/blocks/storage.rs`: crates and rice bag not flammable (`NotFlammable`, FD registers no odds and `ofFullCopy`
  copies none), rice bale faces the clicked face, `#farmersdelight:campfire_signal_smoke` makes a campfire above a
  signal fire (`smoke_source`). Bales keep hay's fire odds (FD's 20/60 entry = hay's) and fall damage × 0.2.
- evidence (FDTest pad 50122–50134,219,49994–50006, creative; `run/shots/m7-03/signal-smoke.png`):
  - all 9 placed; rice bale from the item on a bale's east face → `facing=east`, on the floor → `facing=up`.
  - campfires placed on straw bale and rice bale → `signal_fire=true`; on a crate → `signal_fire=false`.
  - fire beside a crate, the rice bag and the straw bale: after 40 s straw bale burnt, crate and rice bag intact and
    their fires out; the fire then spread to the rice bale (burnt), the rice bag beside it still intact after 80 s.
  - config off (temporary build with `ENABLE_VANILLA_CROP_CRATES_DEFAULT = false`, run in the demo world FDM6 so
    the user's recipe book is untouched): `recipe give` → "Unknown recipe" for carrot/potato/beetroot crate;
    cabbage crate and carrot_from_crate still load (FD gives those no condition). Reverted; FDTest relaunched on the
    config-on build (`carrot_crate` known again).
- not covered: fall damage on bales (hay template behaviour, not exercised); structure rotation of the rice bale's
  declared `facing` (no FD structure places one).
- user data: a `recipe take` during the recipe check removed the user's unlocked `carrot_crate` recipe; it was
  given back at once (`recipe give`, confirmed known after the restart).
- world changes: pad at ~50128,220,50000 (crates, bag, bale fire remains, 3 campfires); player at 22,160,22,
  creative; time 1000. Test items: the pumpkin pies in hotbar slot 3 were replaced by rice bales, then campfires (4).
- next: M7-04 (basket).

## M7-04 Basket — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: `wooden_basket`, `bamboo_basket` declared (oak-planks template, declared 6-way `facing`, `enabled`,
  `waterlogged`; hollow-cube outline/collision per facing, full interaction shape, occludes nothing; fuel 300).
  Code `src/blocks/basket.rs`: block entity (27 slots, `ModContainer` on every face, menu `minecraft:generic_9x3`),
  placement opposite the nearest looking direction (Direction.orderedByNearest port) and waterlogged in water,
  water kept on removal (E-13 as the board), `useWithoutItem` opens the menu, collection tick (TransferCooldown,
  -1 default; 8 after a whole entity goes in; area per facing from Basket.COLLECTION_AREA_SHAPES; slot-order insert
  with merge up to the stack's max; not when full), comparator (getRedstoneSignalFromContainer via the pot's
  helper), not pathfindable, not flammable, client prediction for main-hand clicks (E-28).
- `enabled`: FD writes it in `neighborChanged` with flag 4. The neighbour hook's host cannot write at the pin, so
  the basket's tick reads `has_neighbor_signal` and writes `enabled` (flag 4) before its transfer: same transfer
  outcome (the block update lands before the block-entity tick in vanilla too); clients never see the change,
  as with FD.
- evidence (FDTest pad ~50126,220,49996, creative; `run/shots/m7-04/`):
  - placed from the item looking down north-west at a shallow angle → `facing=east`.
  - item entities summoned in front of the opening: 12 carrots + 5 potatoes collected into slots 0–1; an apple behind
    the basket stays.
  - redstone block beside it: server state `enabled=false` (`execute if block`), client still `enabled=true`; beetroot
    in front not collected; power removed → collected.
  - use opens the 9×3 chest menu titled "Basket" with the contents (`basket-menu.png`); held basket not placed.
  - hopper below pulls items out; comparator 0 when empty, 2 with two full cobblestone stacks (1 + ⌊2/27·14⌋).
  - `setblock … destroy`: 128 cobblestone dropped plus the basket item.
  - five separate sticks in front of an upward bamboo basket: collected one by one over ~1 s.
- deltas: **A30** the menu title is the block's name ("Bamboo Basket"); FD titles both "Basket"
  (`container.farmersdelight.basket`). Engine gap **E-29**: `PlayerRef::open_menu` takes no title. **A31** a hopper
  (or other automation) inserting into an empty basket does not start FD's 8-tick cooldown (BasketInvWrapper); the
  insert path has no mod hook. `farmersdelight:basket` alias not reproduced (accepted in the plan).
- not covered: contents after save/restart (M7-09), waterlogged placement, loot-table fields (no FD structure
  places a basket), exact 8-tick spacing (unit-tested; in-game polling too coarse).
- world changes: bamboo basket with cobblestone + sticks, comparator and wire at ~50126,220,49996; player at
  22,160,22, creative. Test items: wooden baskets 5 in hotbar slot 3 (replacing the campfires).
- next: M7-05 (cabinets).

## M7-05 Cabinets — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: 13 cabinets declared (`tools/decl/blocks.toml.in`, oak-planks template as the baskets; barrel's strength 2.5
  and wood map colour, cherry/bamboo/nether sound types, fuel 300 except crimson/warped; horizontal `facing`,
  `open`). Code `src/blocks/cabinet.rs`: one block entity type for all 13 (27 slots, `ModContainer` on every face,
  `generic_9x3` menu), facing opposite the placer, `useWithoutItem` opens the menu (SUCCESS; client prediction E-28),
  comparator via the basket's signal cache (now shared helpers `basket::{signal, update_signal, forget_signal}`),
  fire odds dropped. **Barrel-template probe (U-15): rejected** — the engine's barrel branch hard-codes barrel sounds
  and a 6-way facing; instead the engine's `mod_container_viewers` hook (vanilla `ContainerOpenersCounter` for mod
  containers) drives `onOpen`/`onClose`: sound `block.cabinet.open`/`close` half a block out of the front (volume
  0.5, pitch 0.9–1.0), `open` written with flag 3, `container_open`/`container_close` game events.
- Custom name (`copy_components minecraft:custom_name` from the block entity, baskets too): the engine keeps the
  placed item's name under the block entity's `components`; `pre_remove_side_effects` records it and `block_drops`
  puts it on the block's own item (item keys resolved at registration).
- evidence (FDTest pad 50145..50155,219,49995..50005, creative; `run/shots/m7-05/`):
  - placed looking south → `facing=north`, looking west → `facing=east`.
  - use opens the 9×3 menu titled "Oak Cabinet" (`cabinet-menu.png`); state `open=true` on server
    (`execute if block`) and client; closing → `open=false` within a tick; subtitles "Cabinet opens" /
    "Cabinet closes" (`subtitles.png`).
  - two cobblestone stacks (slots 0, 26) shown in the menu; comparator → wire power 2; hopper below pulled 6 → power 1;
    `setblock … destroy` dropped all 122 cobblestone plus the cabinet.
  - contents (17 carrots slot 4, 5 cabbages slot 13) identical after quit and relaunch.
  - named "Pantry" cherry cabinet broken in survival → dropped item keeps `minecraft:custom_name "Pantry"`.
- deltas / gaps:
  - **A30 extended**: cabinet menu title is the block's name ("Oak Cabinet"), FD "Cabinet" or the custom name (E-29).
  - **A32** close is heard within a tick of the last viewer leaving; in FD the close comes from the 5-tick
    `recheckOpeners` (FD's manual `startOpen(Player)` keeps the count from reaching 0 on menu close), so up to 5 ticks
    later. The open event's game-event source is none (FD: the player).
  - **E-30 (new)**: `block_drops` gives no block entity (NeoForge's `BlockDropsEvent.getBlockEntity`), and level-side
    breaks (`/setblock destroy`, explosions, pistons) drop before the removal and skip `loot_drops` → **A33** the
    custom name is lost on those paths (player breaks keep it). Same limit applies to the cooking pot's meal item form.
- tooling note: `ctl look`/`place` turn the view without sending the rotation to the server until a mouse move, so
  the placement saw the previous yaw; `ctl mouse-move 2 0` before `place` gives the real facing (not a mod issue).
- incident: in creative, `click-slot` ids address the creative screen; three clicks replaced the user's 3 shepherd's
  pies in hotbar slot 9 with a jungle plank. Restored with `item replace … shepherds_pie 3`.
- not covered: second viewer / LAN `open` state (M7-09), loot-table (`LootTable`) fields (no FD structure places a
  cabinet), flammability in game (same Part as the basket).
- world changes: stone pad at 50145..50155,219,49995..50005 (cabinets removed). Test items left in the user's
  inventory: 3 oak, 1 crimson, 3 cherry cabinets (main slots 16–18). Player returned to 22,160,22, creative;
  random_tick_speed 3. Game left running (restarted three times during the task).
- next: M7-06 (canvas signs).

## M7-06 Canvas signs — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: 68 blocks declared (`tools/decl/blocks.toml.in`, no code): for the plain board and 16 dyes, `<c>canvas_sign`
  (template `minecraft:spruce_sign`, `wall_block` → `<c>canvas_wall_sign`), `<c>canvas_wall_sign`
  (`spruce_wall_sign`, `item = false`), `<c>hanging_canvas_sign` (`spruce_hanging_sign`, `wall_block` →
  `<c>wall_hanging_canvas_sign`), `<c>wall_hanging_canvas_sign` (`spruce_wall_hanging_sign`, `item = false`).
  FD's blocks are WoodType.SPRUCE with ofFullCopy(SPRUCE_SIGN / SPRUCE_HANGING_SIGN). `wood_type`
  `farmersdelight:canvas[_<dye>]` gives the editor FD's `textures/gui/{signs,hanging_signs}/canvas[_<dye>].png`
  (SignEditScreenMixin / HangingSignEditScreenMixin); `sign_text_color = "white"` on both forms of the default
  dark list (gray, purple, blue, brown, green, red, black; SignBlockEntityMixin). Items stack to 64 (FD basicItem()).
  Wall forms have no loot table of their own: the engine's wall-variant fallback (`_wall_` → `_`, leading `wall_`
  dropped) reaches FD's `<c>canvas_sign` / `<c>hanging_canvas_sign` tables, as FD's overrideLootTable does.
- evidence (pad 50145..50155,220,49995..50005; `run/shots/m7-06/`):
  - black canvas sign placed on the floor → `rotation=8`; editor shows the black canvas background
    (`editor-black.png`), text colour `white`; text saved via Done.
  - plain canvas sign used on a wall → `canvas_wall_sign[facing=north]`, text colour `black`.
  - blue hanging canvas sign under a ceiling → `blue_hanging_canvas_sign`, editor with the blue hanging canvas
    (`editor-hanging-blue.png`), colour `white`; on a wall side → `blue_wall_hanging_canvas_sign`, colour `white`.
  - all four boards drawn with their canvas textures (`boards.png`).
  - `setblock … destroy` of the four: drops `black_canvas_sign`, `canvas_sign`, `blue_hanging_canvas_sign` ×2.
  - hanging sign stack of 64 accepted; all 68 ids `setblock` successfully.
- deltas / gaps:
  - **A34** placing a standing canvas sign opens the text editor (the template's SignItem path). FD's item is a
    StandingAndWallBlockItem, which never opens it on placement (the sign is edited by using it). Engine gap
    **E-31**: a sign block's item always opens the editor; `player_open_sign` is a notice and cannot stop it.
    Hanging canvas signs use HangingSignItem in FD too, so they match.
  - **A35 (pending M9-01)**: the dark-background list is static in the declarations (`sign_text_color`), so
    `canvasSignDarkBackgroundList` edits will not change it; to be revisited when the config is wired.
- licence (D7): the boards, editor backgrounds and models are FD's own MIT assets (carried, attributed in
  `LICENSE-farmersdelight`); the models' parents `minecraft:block/template_*sign*` are referenced, not shipped.
- tooling note: `run/ctl close` does not close the sign editor (vanilla oak sign too); `ctl click-at 213 213`
  presses Done at the 1280×720 window.
- not covered: dyeing / glow ink on a canvas sign (vanilla code path), fuel values (vanilla `#minecraft:signs` /
  `#minecraft:hanging_signs` tags list them).
- world changes: pad cleared (leftover M7-05 cabinets removed too). The world time was set to noon to take the
  screenshots (it was night). Player back at 22,160,22, creative; hotbar slot 9 holds the shepherd's pies again.
- next: M7-07 (rug, tatami, mats).

## M7-07 Rug, tatami, mats — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: `tatami`, `full_tatami_mat`, `half_tatami_mat`, `canvas_rug` declared (white-wool template; mats 0.3
  strength and FD's 2-px shape, half mat POPPED, rug 0.2 strength, grass sounds, 1-px shape, no crack overlay and
  no survival rule — FD's CanvasRugBlock is a plain Block; fuel 400/200/100/200; wool fire odds dropped). Code
  `src/blocks/tatami.rs`: tatami placement (facing into the clicked block, paired with an unpaired clicked tatami
  unless sneaking) and unpairing in `updateShape`; full mat placement along the look direction, refused unless the
  cell ahead is replaceable with a floor; mat `updateShape` (partner and floor), creative foot break removing the
  head without drops (`on_destroyed_by_player` → `destroy_block(drop=false)`); half mat facing and floor rule.
- `setPlacedBy` (no `on_block_placed_by` at the pin): done in `entity_place`, which the engine fires from a
  placement preview before the block is written; it writes the partner (paired tatami / mat head, flag 3). The
  engine then asks `state_for_placement` again for the real placement, so both rules accept the partner just
  written (a head of the same facing in front; a clicked tatami already paired toward this cell). Found in game:
  without that, the mat head was written and the foot refused.
- evidence (pad 50145..50155,219,49995..50005, creative unless noted):
  - mat looking south → foot `[facing=south,part=foot]` + head at z+1 `[part=head]`; with stone in the head cell
    the placement is refused (nothing placed).
  - creative break of a foot: both halves gone, no item. Survival break of a foot: one mat dropped (from the head,
    FD's loot is `part=head` only); survival break of a head: one mat, the foot vanishes.
  - tatami placed against a tatami's side → clicked one `[facing=north,paired=true]`, new one
    `[facing=south,paired=true]`; removing one → the other `paired=false`. With Shift held → both unpaired.
  - half mat placed looking south → `facing=north`; floor removed → it breaks and drops itself.
  - rug stays when its floor is removed (FD: no survival rule); sky light under it 15 (propagatesSkylightDown).
- deltas: **A36** the head removed by a creative foot break sends its break particles/sound to the breaker as well
  (FD `levelEvent(player, 2001, …)` skips the breaker; the API has no raw state id for a hand-made level event).
- tooling note: `ctl interact --sneak` / `place --sneak` click before the server sees the sneak; hold Shift
  (`ctl key shift down`) first.
- not covered: crack overlay hidden on the rug (declared `destroy_overlay = false`, not watched mid-break),
  combination step sounds (tag carried).
- world changes: pad cleared and its floor restored. Player back at 22,160,22, creative, hotbar slot 9 restored.
- next: M7-08 (safety net, straw bed, misc).

## M7-08 Safety net, straw bed, misc — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: `safety_net` declared (brown-wool template: carpet's brown map colour, wool sounds; 0.2 strength, FD's
  7–9 px slab shape, `waterlogged`, fuel 200) with `src/blocks/safety_net.rs`: `bounce_restitution` 0.6 (FD
  `Properties.bounceRestitution(0.6F)`), `fall_on` zeroes the fall damage unless the entity suppresses bounce,
  waterlogging and water-on-removal as the basket, fire odds dropped. `fall_on`'s host cannot read the entity, so a
  `server_tick` (start) keeps the set of players holding Shift (`Player.isSuppressingBounce` = `isShiftKeyDown`).
  `straw_bed` declared on vanilla 26.3's own `minecraft:straw_bed` template (FD registers vanilla's StrawBedBlock
  class; Q14) with FD's properties, item stack 16 and compost 0.85. With these, every FD block is declared (only the
  knife items remain, M8).
- evidence (pad, `run/shots/m7-08/`):
  - a cow dropped from y 241 onto the net: bounced to ~224.9, then ~221.7, health 10/10; a control cow dropped on
    stone died.
  - player (survival) from y 233 onto the net: no damage (20/20), bounced to ~223.4. Holding Shift from y 226.5:
    `hurt 2.0 minecraft:fall`, no bounce.
  - straw bed placed from its item: foot + head `[facing=south]`; a stack of 16; sleeping in it works.
- deltas / gaps:
  - **E-32 (new)** the engine's straw-bed sleep rule (`StrawBedBlock.getSleepHeight`: lie on the foot's shape) is
    keyed to the id `minecraft:straw_bed` (`horizon-client/src/bed_geometry.rs`), not the template → **A37** in
    FD's straw bed the sleeper lies at the head's height (y +0.4375, vanilla straw bed +0.375), and a head whose
    foot is gone can still be slept in.
  - **A38** the sneaking test for the net reads the Shift key as of the start of the server tick, not at the landing
    instant.
- incident: the control cow's drops (2 beef, 2 leather) were picked up into the user's inventory; removed with
  `item replace … air` (main slots 19–20). World time: changed by `time set night` + sleeping twice (now morning).
- world changes: pad cleared. Player at 22,160,22, creative; hotbar slot 9 restored.
- next: M7-09 (M7 demo).

## M7-09 M7 demo — PASS — 2026-11-15 — **M7 closed**
- build pin: afe471cf | mod commit: 399cfe3 (no code change in this task)
- single player (FDTest pad 50145..50155,219,49995..50005; `run/shots/m7-09/`): tour of every M7 block family —
  honey glazed ham (one serving taken with a bowl in game, 4 → 3), rice roll medley, gleaming salad, apple pie
  (bites 1), sweet berry cheesecake (bites 2), wooden basket (12 tomatoes, 3 onions), spruce cabinet (40 rice,
  7 bowls), cabbage crate, rice bag, straw bale, red canvas sign with text, paired tatami, full mat (foot+head),
  half mat, canvas rug, safety net, straw bed (`tour-before.png`). Per-feature behaviour: the M7-01…M7-08 entries.
  - full restart (`ctl quit`, `launch.sh FDTest`): the 20 block states, both containers' `Items` and the sign text
    read before and after are identical (`before.txt`, `after.txt`, `diff` empty).
- dedicated server (re-exported with this build; clients Alice and Bob refreshed from it):
  - Alice opens the oak cabinet → Bob reads `open=true`, sees the open model and hears "Cabinet opens"
    (`bob-cabinet-open.png`); Alice closes → Bob reads `open=false`.
  - both open it: stays `open=true` when Alice closes, `open=false` only when Bob closes too (opener count).
  - Alice takes a roast chicken serving with a bowl → Bob reads `servings=3`; Alice eats a chocolate pie slice →
    Bob reads `bites=1`, sees both and hears "Eating" (`bob-feast-pie.png`).
- incidents: (1) a teleport to z 49993 was just off the pad; the creative player fell to the ground (y 70), no
  damage; back on the pad at once. (2) Around 18:47 the single-player game and Alice's client exited with no save
  and no crash report or OOM entry (window focus switched between the terminal and the game at that time); the last
  save had my test bowl and a ham serving in the user's inventory and a bowl/stone slot swap. Relaunched FDTest,
  restored the inventory to the pre-test baseline (hotbar slot 9 = 3 shepherd's pies, main slot 15 = 1 stone,
  ham removed), player back at 22,160,22, saved and quit: **the game is left closed**.
- world changes: the M7 tour stays on the FDTest pad at 50145..50155,220,49995..50005 (far from the player); the
  server world FDServer has a stone pad at -536..-526,80,66..74 with a cabinet, a roast chicken and a chocolate pie.
- **M7 closed.** Open M7 items: A30/E-29 menu titles, A32 cabinet close timing, A33/E-30 custom name on level-side
  breaks, A34/E-31 sign editor on placement, A35 (M9-01), A36, A37/E-32 straw bed sleep height, A38.
- next: M8-01 (PORT_PLAN §11).

## M8-01 All knives — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: copper/iron/diamond/netherite/golden knives declared in `tools/decl/items.toml.in` as FD's
  `knifeItem(material)` (damage 0.5 + material bonus, speed −2.0, tool rules incorrect-for-material / knife mineable at
  material speed / instant-mine tag, weapon 1, enchantability and repair tag from the material, netherite
  `damage_resistant #minecraft:is_fire`). Vanilla 26.3 material values read from the 26.3 datagen components report
  (`<m>_sword`: damage = 3 + bonus, durability, enchantable, repairable; `<m>_pickaxe`: speed, incorrect tag) —
  copper 190/5/1/13, iron 250/6/2/14, diamond 1561/8/3/10, netherite 2031/9/4/15, gold 32/12/0/22. New
  `tools/test_knife_decls.py` checks all six against these. `src/items/knife.rs`: the six item keys (FD tests
  `instanceof KnifeItem`, not the tag) and `living_knockback` → strength − 0.1F when the attacker's main hand
  holds a knife. Sweeping Edge needs no code: knives are not in `#minecraft:enchantable/sweeping`.
- evidence (FDTest pad 50200..50220,219,49990..50010, removed afterwards; `run/shots/m8-01/`):
  - held attack damage / speed (`attribute get`): flint 2.5, copper 2.5, iron 3.5, diamond 4.5, netherite 5.5,
    golden 1.5; speed 2.0 for all. Durability (inventory report) 131/190/250/1561/2031/32.
  - `/enchant` Sweeping Edge → "cannot support that enchantment" on all six; Sharpness and Looting accepted. Anvil
    (survival): iron knife + Sweeping Edge book → no result, red cross (`anvil-sweeping.png`); + Sharpness book →
    Sharpness I knife, cost 1.
  - lava: netherite knife item survived, diamond knife item burned. Smithing table: template + diamond knife +
    netherite ingot → netherite knife (`smithing.png`). Recipes copper_knife, netherite_knife_smithing,
    iron_nugget_from_smelting_knife known to `/recipe give`.
  - knockback: a pig (slowness/resistance) hit by `damage … player_attack by @s` from the same spot, 3 rounds each:
    bowl in hand → pushed 1.9886 blocks every time, iron knife → 1.3610 every time; ratio 0.684, vanilla motion
    for strength 0.4 vs 0.3 predicts 0.679.
- deltas: **A39** FD takes the knife from the victim's kill credit (`getKillCredit`: last hurting player first);
  the engine's `living_knockback` names the hit's attacker (falling back to the last hurt-by mob). Differs only when
  a knife-holding player hurt the entity and another entity then pushes it.
- incident: the first teleport to the pad landed before its chunk had loaded; the creative player fell to the
  ground (y 63) unhurt and was moved onto the pad after the fill.
- world changes: none left (pad, anvil, smithing table removed). User inventory restored (hotbar slot 9 = 3
  shepherd's pies, test books/knives removed), XP back to 11 levels, survival at 22,160,22, slot 8 selected.
  The game is left running (FDTest, agent mode). It was raining at night.
- next: M8-02 (knife interactions).

## M8-02 Knife interactions — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: `src/items/knife.rs`:
  - `KnifeItem#useOn` as an `ItemPart::use_on` on the six knives: a knife (`#farmersdelight:tools/knives`) on
    `minecraft:pumpkin` plays `block.pumpkin.carve`, sets `carved_pumpkin[facing]` (clicked side; top/bottom →
    toward the player), throws 4 pumpkin seeds 0.65 out of that face with FD's motion, damages the knife by 1.
    Client prediction via `use_item_on_block` (ItemAfterBlock → SUCCESS, E-28 pattern).
  - `KnifeEvents#onCakeInteraction` (Fabric `UseBlockCallback`) as `right_click_block`, decided the same way on
    both sides, world changed on the server only: spectators pass; candle cakes (`#farmersdelight:drops_cake_slice`)
    → `cake[bites=1]`, the candle cake's own loot popped (`block_drops` + `pop_resource` = `Block.dropResources`),
    a slice; cake → bites + 1 or removed after bites 6, a slice at x + bites·0.1 with motion −0.05; wool break
    sound 0.8/0.8. Result SUCCESS, CONSUME for a plain cake on the server, as FD.
  - pie slicing (M7-02) and colony harvest (M5-07) already go through the knife tags and now cover every knife.
- evidence (FDTest pad at 50200..50220,219,49990..50010 under `forceload`, survival, iron knife;
  `run/shots/m8-02/`):
  - pumpkin, front (north) face → `carved_pumpkin[facing=north]`; top face while looking south → `facing=north`;
    east face → `facing=east`; 4 pumpkin seeds each, dropped on the carved side; knife 250 → 247 over three carvings.
  - cake: 7 cuts → bites 1…6, then air; 7 cake slices; knife undamaged (as FD). (After 4 bites the cake's shape
    no longer covers the block centre; the clicks were aimed at the remaining part.)
  - lit red candle cake cut while holding Shift → `cake[bites=1]`, one red candle and one slice dropped, subtitle
    "Block broken" (`candle-cake-cut.png`).
  - apple pie with the iron knife → bites 1, one slice; red mushroom colony age 3 → age 0, 3 red mushrooms, knife −1.
- deltas: none new. (FD also runs the cake change on the client as a prediction; here the client only answers
  SUCCESS and the server's block update follows, no visible difference.)
- world changes: none left (pad removed, forceload removed). User inventory restored, survival at 22,160,22,
  slot 8 selected. The game was restarted once (rebuild) and is left running.
- next: M8-03 (cutting board completion).

## M8-03 Cutting board completion — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did (`src/blocks/cutting_board.rs`):
  - sneak tool carving (`ToolCarvingEvent#onSneakPlaceTool`) as `right_click_block`, both sides deciding alike:
    Shift held, main-hand stack with a `minecraft:tool` component (patch or default), a trident or shears, on an
    empty board → stored, `IsItemCarved` 1, one used outside creative, `block.wood.place` 1.0/0.8. A carved board
    takes no items and cuts nothing until emptied by hand. Render: trident pose 135° added (pickaxes/hoes 225°,
    others 180° were there).
  - Fortune read from the tool's `minecraft:enchantments` (fortune bonus 0.1 per level, config default).
  - processing sound for a block item on the board: its block's default-state break sound at pitch 0.8.
  - comparator: `has_analog_output`, `floor(count / min(64, max) · 14) + 1`, kept per position whenever the slot
    changes (clicks, cuts, carving, automation).
  - `contents_changed` (block Part): automation (hoppers, any side) refreshes the client mirror (`Inventory`), the
    comparator level and syncs — before, a hopper-fed item was invisible on the client.
  - dispenser (`CuttingBoardDispenserMixin`/`CuttingBoardDispenseBehavior`) as `dispense_method` on the board:
    with `enableCuttingBoardDispenserBehavior` (default on; config wiring M9-01) any dispensed stack is used as the
    tool; a cut → click (1000) and the stack comes back damaged by 1; no recipe / empty board → fail sound (1001),
    stack kept; vanilla's dispense never runs. `cut` takes an optional player (no stat, overlay or trigger without).
- evidence (FDTest pad under forceload, survival; `run/shots/m8-03/`):
  - Shift + iron axe on the empty board → `IsItemCarved:1b`, axe stored and drawn upright, hand empty, subtitle
    "Block placed" (`axe-carved.png`); empty-hand click → axe back, `IsItemCarved:0b`.
  - oak log on the board, iron axe → stripped oak log + tree bark, axe 250 → 249, subtitle "Axe strips"
    (`log-stripped.png`).
  - hopper above (64 beef): board 6 / 11 / 16 → comparator 2 / 3 / 4; beef drawn on the board (`hopper-beef.png`);
    hopper on the east side inserted 8; hopper below pulled 11, board 30 → comparator 7.
  - dispenser (facing the board, iron knife): two pulses → 4 minced beef, knife back in the dispenser with damage
    2; a stick → no cut, stick kept.
  - Fortune: 50 dispenser cuts of gravel each — diamond shovel 7 flint (p 0.1, expected 5), Fortune III shovel 20
    flint (p 0.4, expected 20).
  - all 117 cutting recipes: shape/tool-form unit tests (`every_reference_recipe_has_a_valid_shape`,
    `tool_form_counts`); in game knife, axe_strip and shovel_dig recipes used.
- deltas: **A40** a dispenser's tool loses 1 durability per cut without Unbreaking's chance (the API has no
  entity-free `hurtAndBreak`).
- not covered: config off (M9-01 wires the config), trident carving render (angle coded, not looked at).
- world changes: none left; user inventory/XP restored, survival at 22,160,22. Game left running.
- next: M8-04 (backstabbing).

## M8-04 Backstabbing — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: `src/enchant.rs`. The carried enchantment JSON loads as is (`/enchant … farmersdelight:backstabbing 3`
  works, the component is stored). `living_incoming_damage`: damage > 0, a causing entity, a source position, the
  hit's weapon with Backstabbing, attacker in the back cone (`isLookingBehindTarget`: normalised offset, y zeroed,
  dot with the victim's view < −0.5) → damage × (1.4 + 0.2·(level − 1)). The crit sound
  (`entity.player.attack.crit`, BLOCKS, 1/1) at the attacker's hit position is queued and played at the end of the
  server tick (the hook's level host cannot play sounds).
- evidence (pad, NoAI pig with 200 health looking north, survival player, fully charged hits; `run/shots/m8-04/`):
  - iron knife (3.5) with Backstabbing III: front 3.5, 3.5; behind 6.3, 6.3 (×1.8); subtitle "Critical attack"
    (`behind.png`).
  - Backstabbing I: front 3.5, behind 4.9 (×1.4), from the side 3.5.
- deltas: **A41** the effect's value is computed from the shipped JSON's numbers (`multiply` by
  `linear(1.4, 0.2)`), not by evaluating the enchantment's effect list, so a data pack changing the
  `farmersdelight:backstabbing` effect is not followed; the weapon is the hook's (the direct entity's) rather than
  `source.getEntity().getWeaponItem()` (same for melee); the crit sound is heard at the end of the tick (same tick).
- world changes: none left; user inventory restored, survival at 22,160,22. Game left running.
- next: M8-05 (loot hooks).

## M8-05 Loot hooks — PASS (rows checked below; smaller samples than the plan's n≥200) — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: `src/loot.rs`.
  - `loot_table_load` writes FD's `LootTableEvents.MODIFY` builders as 26.3 loot JSON (found in game: 26.3 uses
    `type` discriminators, one `condition` per pool/entry — several become `all_of` —, `modifier` for functions,
    `match_block` with `blocks`/`state`, entity predicates `minecraft:equipment` / `minecraft:flags`; the first,
    pre-26.3-style version made the pumpkin table drop nothing): 14 chest tables get a pool nesting FD's chest table
    (`generateFDChestLoot`, default on); knife scavenging (chicken feather, hoglin/pig ham and smoked ham with the
    fire flag, pig with `random_chance_with_enchanted_bonus` 0.5 / looting linear 0.6+0.1, rabbit hide, shulker
    shell, spider/cave spider string); pumpkin (old pools only without a knife or with Silk Touch, 4 slices
    otherwise); pastry slicing for apple/chocolate/pumpkin pie, cheesecake and cake (old pools guarded by "not a
    knife", one pool per bites value giving max − bites slices); straw (grass 0.3, sandy shrub 0.3, rice panicles
    age 3, wheat age 7, all with `#farmersdelight:straw_harvesters`).
  - FD's `MODIFY_DROPS` rows become tag-gated pools: every entity table gets "leather if the attacker holds a knife
    and this is `#farmersdelight:drops_leather`", every block table "7 cake slices if the tool is a knife and the
    block is `#farmersdelight:drops_cake_slice`" (the engine's loot predicates read both tags), which is the same
    as FD's "its own table rolled" check.
  - `loot_condition` `farmersdelight:can_item_perform_ability` (ability → its tag on the context's tool; it was
    unanswered before, i.e. always false, so shears on wild crops never gave the crop); `loot_function`
    `farmersdelight:smoker_cook` (the smoking recipe's result × count, from a table rebuilt on each server data
    load; no carried table uses it).
- evidence (FDTest pad; `/loot spawn … mine` for blocks and chests, real knife kills for mobs — `/loot … kill`
  in this engine carries no attacker, so it cannot test attacker rows):
  - lime and red candle cake + knife → 7 cake slices + candle; by hand → candle only.
  - pumpkin + knife → 4 pumpkin slices; by hand → pumpkin.
  - apple pie (bites 2) + knife → 2 slices, by hand → nothing; cake (bites 3) + knife → 4 slices, by hand → nothing.
  - wheat age 7 + knife ×5 → 5 straw; age 6 → none. Short grass + knife ×100 → 37 straw (p 0.3, 99% 18–42);
    by hand ×30 → none. Rice panicles age 3 + knife → rice + straw. Sandy shrub + shears → the shrub; + knife ×50
    → 12 straw (p 0.3, 99% 7–24); by hand → nothing. Wild cabbages + shears → wild cabbages; by hand → seeds.
  - chests ×15 each: plains house (tomato seeds, onions), simple dungeon (cabbage/tomato seeds, rope), shipwreck
    supply (+ rice), butcher (bacon, minced beef, mutton chops, ham, flint and iron knives).
  - knife kills: 6 rabbits all ≥ 1 hide (vanilla alone: 0 half the time); 6 + 6 cows all ≥ 1 leather, control
    with an iron sword had a cow with none; 20 pigs → ham in 8 (p 0.5, 99% 4–16), 8 burning pigs → smoked ham in
    3, never raw ham, plus 6 more pigs ham in 4.
- not covered in game: spider/cave spider string, shulker shell, hoglin (hostile; the world is Peaceful — same
  pool shape as the rabbit/pig rows), chicken feather (vanilla drops 0–2 feathers, not separable at small n),
  `smoker_cook` (no table uses it), Looting raising the ham chance.
- incident: in an early kill script the player stood close enough to pick up a feather and a raw chicken
  (removed again); the test kills raised the player's XP to 15 levels, set back to 11 levels + 13 points (the
  lifetime XP total stays 317 instead of 200).
- world changes: none left. Game left running.
- next: M8-06 (rotten tomato).

## M8-06 Rotten tomato — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: entity type `farmersdelight:rotten_tomato` declared on the snowball template (`tools/decl/entities.toml.in`;
  size 0.25, tracking 4, interval 10 are the snowball's; `damage` 0 = FD's `hurt(thrown, 0)`). `src/items/rotten_tomato.rs`:
  `use_item` (FD's throw sound NEUTRAL 0.5 / `0.4/(r·0.4+0.8)`, `spawn_projectile_from_rotation` at 1.5 with 1.0
  spread, use stat, one used outside creative; SUCCESS client / CONSUME server), `dispense_item`
  (`ProjectileDispenseBehavior` default: 0.7 in front of the centre, `(x, y+0.1, z)` × 1.1 with uncertainty 6,
  launch sound 1002), `projectile_impact` (FD's hit sound once per impact, twice when an entity is hit:
  `onHitEntity` + `onHit`).
- evidence (pad; `run/shots/m8-06/`):
  - thrown at a pig: stack 16 → 15; subtitles "Rotten Tomato flies", "Pig hurts", "Rotten Tomato splatters"
    (`hit.png`); pig health stays 10. A slowed pig with AI was pushed 1.99 blocks away from the thrower, no damage.
  - dispenser facing east (5 tomatoes): pulses leave 4, 3; the projectile is a `farmersdelight:rotten_tomato`
    flying east (+3.3 blocks per 0.1 s poll, slight drop); `/summon farmersdelight:rotten_tomato` works.
- deltas: **A43** the impact particles are the snowball's 8 item crumbs (FD's `handleEntityEvent` draws 12); the
  hit-sound pitches and the dispenser's spread draw from the level random, not the projectile's.
- not covered: raider anger and the `hit_raider_with_rotten_tomato` advancement (M8-07), LAN view (M8-08).
- world changes: none left. Game left running.
- next: M8-07 (advancements).

## M8-07 Advancements — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: nothing new needed for 19 of the 21 (carried data + the engine's vanilla triggers + the M1
  `farmersdelight:use_cutting_board` trigger). Two FD criteria come from vanilla triggers that the engine does not
  fire for mod-side actions, and a mod cannot fire a `minecraft:` trigger (`trigger_criterion` refuses them):
  - **E-33** `minecraft:placed_block` is not fired when a mod places a block from its own code (the skillet's
    sneak placement, `SkilletItem#place` → `BlockItem#place`).
  - **E-34** `minecraft:consume_item` is fired only for consumables, not for any finished item use
    (`ServerPlayer#completeUsingItem`: the skillet's held cooking).
  - workaround **A44**: `items::skillet::grant_criterion` runs the console's
    `advancement grant <player> only farmersdelight:main/place_skillet skillet` (resp. `use_skillet`) at those
    two moments. Other advancements (data packs) with those triggers for the skillet are not fired. (A UUID
    target is refused by this engine's `advancement` command, so the player's name is used.)
- evidence (test world FDM6, Normal; all FD advancements revoked first: "Revoked 21 advancements"; checked with
  `execute if entity @s[advancements={…=true}]`):
  - `/give`: root (seeds), craft_knife, get_fd_seed, get_ham, get_mushroom_colony, get_rich_soil, harvest_straw,
    obtain_netherite_knife.
  - placements by the player: plant_all_crops (wheat, beetroots, carrots, potatoes, melon and pumpkin stems,
    cabbages, onions, budding tomatoes on farmland; mushrooms on mycelium; chorus flower on end stone; cocoa on a
    jungle log; glow berries under stone; nether wart on soul sand; sweet berries on grass; sugar cane beside
    water; kelp and rice in water), plant_rice, place_campfire, place_cooking_pot, place_feast (gleaming salad
    block), place_organic_compost, place_skillet (after the fix).
  - use_cutting_board (beef cut with a knife), harvest_ropelogged_tomato (ripe tomatoes on rope picked → age 0),
    hit_raider_with_rotten_tomato (NoAI pillager, health stays 24), eat_nourishing_food and master_chef (all 27
    meals eaten in survival, hunger drained with a Hunger effect between), use_skillet (beef cooked in the held
    skillet beside a campfire; after the fix).
  - all 21 granted at the end; all 203 recipe advancement ids are known to `advancement` (an unknown id answers
    "Unknown advancement").
- world changes: FDM6 (test world) has a planting pad at 10..34,99,1330..1348 and the player there in survival;
  FDTest untouched in this task.
- next: M8-08 (M8 demo).

## M8-08 M8 demo — PASS — 2026-11-15 — **M8 closed**
- build pin: afe471cf | mod commit: (this commit; code as of eb85bb6)
- recorded runs of every M8 Accept item: M8-01…M8-07 entries above (knife stats, anvil, knockback; carving,
  cakes; board carving, recipes, fortune, comparator, hoppers, dispenser; backstabbing ratios; loot rows;
  rotten tomato; 21 advancements). Screenshots under `run/shots/m8-0*/`.
- restart (test world FDM6; `run/shots/m8-08/`): a trident carved into a board (Shift + use) → `IsItemCarved:1b`,
  trident in `Items`/`Inventory`, drawn upright (`trident-before.png`); after `ctl quit` + relaunch the same
  NBT and the same render (`trident-after.png`). All 21 FD advancements still granted after the restart.
- second client (dedicated server re-exported with this build via `native-build --server-output`, synced over
  `run/server-engine`; clients refreshed from `native-afe471c-9c75d86b611b`): Alice throws a rotten tomato at a
  pig; Bob's client shows the pig's hurt flash and the subtitles "Rotten Tomato flies", "Pig hurts", "Rotten Tomato
  splatters" (`bob-tomato-impact.png`). Server and clients stopped afterwards.
- **M8 closed.** New in M8: deltas A39–A44, engine gaps E-33 (placed_block from mod code), E-34 (consume_item only
  for consumables). Open M8 items: A40 (dispenser tool ignores Unbreaking), A41 (backstab value from shipped
  numbers), A43 (8 crumbs, level random), A44 (skillet criteria via grant); not run: hostile-mob scavenging rows
  (Peaceful FDTest), Looting on ham chance, `smoker_cook` (unused by data), config-off paths (M9-01).
- world changes: FDM6 (test world) keeps the planting pad and boards at 10..34,99..103,1330..1348; the player
  there in creative. FDTest: none (inventory/XP restored earlier). The game is left closed.
- next: M9-01 (PORT_PLAN §11).

## M9-01 Config — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: `src/config.rs` rewritten: FD's two files as the Fabric build keeps them (mlconfigs `FabricConfigHolder`):
  pretty-printed JSON, `#README` + one object per category (`settings`, `farming`, `crafting`, `overrides` with
  nested `stack_size`, `world`, `debug`; client file `client`), 19 common + 3 client keys with FD's defaults and
  ranges. Registered with `register_configs` (Common + Client); `config_loading` and `config_reloading` read the
  text over the current values with the reference's per-entry rules (missing key/category or unreadable value keeps
  the value; double out of range → default; Gson's lenient reads: `"TRUE"` → true, a number as a boolean → false,
  `"0.25d"` → 0.25, one-element arrays unwrap; a non-array list keeps its value; an unreadable list element stops
  the list) and write the normalised file back when it differs (`saveConfig`), via `Host::config_files`. Not
  synced (Q6): each side reads its own file.
  Consumers wired: every existing reader now takes the loaded value; new ones: `world.generateFDChestLoot`
  (loot.rs), `farming.defaultTomatoVineRope` (rope.rs `rope_state_for_vine`: FD's rope with connections, another
  block's default state, unknown id → air as `DefaultedRegistry` gives), `settings.canvasSignDarkBackgroundList`
  (new `blocks/canvas_sign.rs`: a player's placement sets both sides' colour with `data merge` at the tick's end —
  the block entity is made after `entity_place`), `soupItemList` matched as `Identifier.parse` does (bare path →
  `minecraft:`). Getters ready for M9-02 (`enableCookingPotRecipeBook`), M9-03 (two overlay keys), M9-04
  (`enableFoodEffectTooltip`). Village keys are read but stay inert (A26/E-26).
- startup-only / reload-only keys (documented in `config.rs`): `overrides.*` item-default keys and
  `stack_size.*` are read when item defaults are built; `enableVanillaCropCrates` and `generateFDChestLoot` act on
  the next data load (`/reload`); every other key is read where it is used, so a file edit applies within ~1 s.
- evidence (test world FDM6, pad 10..20,99..104,1340..1348; file `config/ported_farmersdelight-common.json` edited
  while the game ran; each check run with the key changed and back):
  - files written on first start in FD's layout (both files; default text = unit-tested render).
  - `enableRopeReeling` false: Shift + empty-hand use leaves the 3-rope column; true: bottom rope reeled (air).
  - `defaultTomatoVineRope` `minecraft:stone`: a broken `tomatoes_on_rope` leaves stone; `enableTomatoRopePermanence`
    false: leaves air; both back to default: leaves `farmersdelight:rope`.
  - `enableVanillaCropCrates` false + `/reload`: `recipe give … carrot_crate` → "Unknown recipe"; true + `/reload`:
    known again. `generateFDChestLoot` false + `/reload`: 40 rolls of `chests/abandoned_mineshaft` → 0 FD items
    (147 others); true: 45 FD items in 40 rolls.
  - `enablePumpkinPieSneakToPlace` true: plain use places nothing, Shift places `farmersdelight:pumpkin_pie`; false:
    plain use places it.
  - `enableCuttingBoardDispenserBehavior` false: the dispenser shoots the flint knife out, beef stays on the board;
    true: the beef is cut and the knife takes 1 damage.
  - `richSoilBoostChance` 0.0: 5 cave vines under rich soil keep `berries=false` (random_tick_speed 200, 4 s); 1.0:
    all 5 `berries=true`.
  - `enableTomatoVineClimbingTaggedRopes` (test datapack adding `minecraft:iron_chain` to `#farmersdelight:ropes`,
    removed afterwards): true → the vine climbs the chain; false → the chain stays, FD's rope is still climbed.
  - `cuttingBoardFortuneBonus` 1.0 with a Fortune III shovel: 6 gravel cuts → 6 flint; 0.0: 10 cuts → 1 flint.
  - `canvasSignDarkBackgroundList` `["white"]`, read at start (no edit after the restart): a player's white canvas
    sign gets `front_text.color`/`back_text.color` "white", a red one "black" (`run/shots/m9-01/`).
- engine gap **E-35**: config files load after item default components are built (the window reads Client/Common
  configs after its initial load, `loading_window.rs` "Finish Setup -> Config"; `modify_default_components` runs
  once per item at the load boundary and its `DataLoadContext` offers settings, not config files), so the
  item-default keys cannot reach their consumer. Observed: `enableStackableSoupItems` false across a restart, mushroom
  stew still stacks to 16. A settings mirror was considered and rejected (adds Options rows FD lacks, applies one
  restart late, no list kind).
- deltas: **A45** the files are `ported_farmersdelight-common.json` / `-client.json` (the engine accepts only names
  owned by the mod id), not `farmersdelight-*.json`. **A46** an edit to the file applies while the game runs (FD
  reads the file once; only its YACL/Cloth screen applies live); an unparseable `defaultTomatoVineRope` leaves air
  instead of throwing. **A47** (E-35) `enableVanillaStrawBedRename` (already inert, A17), `enableVanillaSoupExtraEffects`
  and `enableRabbitStewBuff` for item defaults, `enableStackableSoupItems`, `soupItemList` always have their defaults.
  **A35** narrowed: canvas signs placed by players follow the configured list; signs made by commands or structures
  keep the default list (static `sign_text_color`).
- not covered: client keys (no consumer before M9-03/M9-04); a dedicated server's own file (M9-08 LAN).
- world changes: FDM6 (test world): pad 10..20,99..104,1340..1348 holds rich soil, cave vines, signs, pies, board +
  dispenser, tomato vines; random_tick_speed 3, time noon. Config files back to defaults. FDTest untouched. Game closed.
- next: M9-02 (recipe book).

## M9-02 Recipe book — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: `src/recipes/book.rs`. Book type `farmersdelight:cooking` registered (`recipe_book_type`); the pot menu bound
  to it with `menu_recipe_book` (layout `Crafting` 3×2 over the six inputs, result role = output slot 8) and a
  frozen screen = the pot's screen + book + toggle at (5, 34) (`CookingPotScreen#getRecipeBookButtonPosition`:
  `leftPos + 5`, `height/2 − 49`); the block's `screen` hook returns the same value. `recipe_book_categories` builds
  three categories from the parsed pot recipes by `recipe_book_tab`: `farmersdelight:cooking_meals` (icon vegetable
  noodles), `cooking_drinks` (apple cider), `cooking_misc` (dumplings), search group
  `farmersdelight:cooking_search`; each recipe a Shapeless display (ingredients, result stack, station cooking pot)
  with placement cells (a `fabric:any` slot merged into one ingredient). `enableCookingPotRecipeBook` off: the
  pot closes an open book when it opens (`CookingPotScreen#init` toggles it shut; the button stays usable).
  Decision: the `send_recipe_book_values` payload is not needed — the engine syncs custom book settings itself.
  FD's `farmersdelight:cutting` category is not declared (no screen shows it; the engine needs a target menu).
- evidence (`run/shots/m9-02/`):
  - single player (FDM6): toggle drawn at FD's spot (`pot-closed.png`); book opens with search + 3 tabs
    (`book-open.png`); with the player's 20 unlocked pot recipes: Drinks 1, Meals 14, Misc 5; after
    `recipe give @s *`: Drinks 2, Meals 20, Misc 6 (= the 28 files' tabs), search 20 + a second page; a craftable
    recipe has the white frame (`misc-tab.png`). The open state survived closing the pot and a game restart.
  - dedicated server (re-exported with this build, Alice): clicking Tomato Sauce with 4 tomatoes moves 2 into input
    slots 0 and 1 (`server-placed.png`); clicking Cabbage Rolls without ingredients shows ghost cabbage and raw
    pork in the grid and the result ghost in the output slot, the tomatoes go back (`server-ghost.png`).
    `enableCookingPotRecipeBook` false in the server's file: reopening the pot → book closed, the toggle still
    opens it; true: the open state is kept.
- engine gap **E-36**: in single player (window on the integrated server thread) clicking a recipe in a mod book
  does nothing — no placement and no ghost, nothing logged, also after reopening the pot 3 times; the same build
  places and shows ghosts for a dedicated server's client. The client drops the click when no catalogue was
  negotiated (`request_mod_book_recipe` needs a placement window; the server answers catalogue requests through
  `mod_book_menu_snapshot`, which reads the LAN hub's guest). No mod-side route (the catalogue handshake is engine
  internal). Not E-08: the E-08 retry (`f87fde4`) is present at the pin as `apply_screen_with_mod_book`.
- deltas: **A48** (E-36) no recipe placement or ghost in single player. **A49** book presentation from the engine:
  tab order Drinks, Meals, Misc (categories ordered by id; FD: Meals, Drinks, Misc); one Misc icon (FD shows
  dumplings and tomato sauce); vanilla filter-button sprites and "Craftable" wording (FD: `cooking_pot_*` sprites,
  "Cookable"); result ghost in the output slot (FD: the meal display) and no ghost for the serving container
  (FD fills slot 7 when empty).
- world changes: FDM6 cooking pot at 12,100,1343; FDServer: a cooking pot at −532,81,70 and Alice's tomatoes. All
  games stopped.
- next: M9-03 (HUD overlays).

## M9-03 HUD overlays — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: `src/client/hud.rs`: two GUI layers `farmersdelight:comfort` and `farmersdelight:nourishment` registered
  above vanilla's `minecraft:player_health` (FD draws them after the health and food bars); `gui_layer_draw` ports
  `drawNourishmentOverlay` (background + gilded full/half icons from `textures/gui/fd_icons.png`, the
  natural-healing set at +18 when hurt with food ≥ 18, jitter at zero saturation) and `drawComfortOverlay` (the
  sheen over the bottom heart row, frame from `ticks % 50`, jitter at ≤ 4 health, regen bob), each gated by
  `shouldRenderOverlay` (survival/adventure; the engine skips layers when the HUD is hidden) and its client config
  key (`enableNourishmentHungerOverlay`, `enableComfortHealthOverlay`, read live). GUI ticks are counted in
  `client_tick_post`; the jitter uses a `java.util.Random` port seeded `ticks * 312871` (unit test against values
  printed by the system JDK).
- evidence (FDM6, survival; `run/shots/m9-03/`): Nourishment → the ten drumsticks gilded
  (`nourishment-full-crop.png`); `enableNourishmentHungerOverlay` false in the client file while playing → plain
  drumsticks (`nourishment-off-crop.png`), back on → gilded. Comfort with saturation 0.0 and 14/20 health → the
  sheen moves across the hearts frame by frame (`comfort-frames.png`: columns 0, 2–3, 6, 9); at full health
  nothing is drawn.
- deltas: **A50** the absorption amount is not readable on the client, so the Comfort overlay's row count ignores
  absorption hearts (it sits on the wrong row when absorption adds a row). A15 (natural regeneration assumed on)
  applies to the gilded set's healing variant.
- not covered: LAN guest view (M9-08).
- world changes: FDM6 player back in creative, effects cleared, health/food restored. Game running.
- next: M9-04 (tooltips).

## M9-04 Tooltips — PASS — 2026-11-15
- build pin: afe471cf | mod commit: (this commit)
- did: `src/items/tooltips.rs` + generated `src/items/food_tooltips.rs` (`tools/gen_food_tooltips.py [--check]`,
  from each item's `minecraft:apply_effects` in `tools/decl/items.toml.in`; the item lists are ModItems'
  `ConsumableItem(…, hasFoodEffectTooltip, hasCustomTooltip)` flags). Item parts on 46 effect foods, 3 custom-line
  drinks (milk bottle, hot cocoa, melon juice: blue `tooltip.farmersdelight.<path>`) and the two pet foods (gray
  "When fed to …" + ` effect[ potency][ (mm:ss)]`); `addFoodEffectTooltip` lines (`potion.withAmplifier`,
  `potion.withDuration`, category colour, "When Applied:" with Speed/Absorption modifiers as vanilla's
  `createModifiers`, `#.##` amounts); the global `item_tooltip` appends the pumpkin pie's Placeable / Placeable
  when sneaking line and the vanilla soups' Nourishment line (TooltipEvents). All behind
  `enableFoodEffectTooltip` (client, live); the soup line also needs `enableVanillaSoupExtraEffects`.
- evidence (FDM6, chest of test foods; `run/shots/m9-04/`): Apple Cider "Absorption (01:00)", blank, "When Applied:",
  "+4 Max Absorption" (`apple_cider.png`); Milk Bottle "Clears 1 Effect" blue; Beef Stew "Nourishment (03:00)";
  Dog Food and Horse Feed lists ("Speed II (05:00)") (`montage1.png`, `montage2.png`); Mushroom Stew "Nourishment
  (03:00)" and Pumpkin Pie "Placeable" after the advanced lines, as Fabric's callback appends; key off → no FD
  lines (`off-montage.png`).
- deltas: **A51** durations use 20 ticks per second (FD passes the level's tick rate, changed by `/tick rate`); the
  effect category colours are a fixed list for the effects FD's foods use (harmful red, others blue).
- next: M9-05 (particles & sounds).

## M9-05 Particles & sounds — PASS — 2026-11-16
- build pin: afe471cf | mod commit: (this commit)
- did: the one sound the port never played, `block.skillet.sizzle`: `SkilletBlock#animateTick` as the skillet's
  client block-entity tick (heated and holding a stack, 1 tick in 10, BLOCKS, volume 0.4, pitch 0.9–1.1) and
  `SkilletItem#onUseTick` as the skillet item's `on_use_tick` on the client (cooking held food, 1 in 50,
  PLAYERS). FD's flip branch (sizzle in the last 3 ticks of a flip, add_food at FLIP_TIME−8) has no trigger in the
  port: nothing sets the flip timestamp (FD neither, outside its own flip input).
- checklist of the 20 events (subtitles, audio on; `run/shots/m9-05/`, earlier entries cited):
  - [x] block.cabinet.open / close — "Cabinet opens", "Cabinet closes" (`cab-close-crop.png`; M7-05).
  - [x] block.cooking_pot.boil — "Cooking Pot boils" (`pot-steam.png`; M3-09).
  - [x] block.cooking_pot.boil_soup — played instead of boil while a meal is stored (code, M3-09); FD gives it no
    subtitle, so it is not visible in the list.
  - [x] block.cutting_board.knife_cut / place_item — "Knife cuts", "Item placed on board" (M1-09; now
    `subtitles-checklist.png`).
  - [x] block.cutting_board.remove_item — "Item removed from board" (`subtitles-checklist.png`).
  - [–] block.cutting_board.carve_tool, block.food.slice, block.food.take_portion — registered with subtitles; FD
    never plays them (only ModSounds/SoundDefinitions name them).
  - [x] block.rope_fence_gate.open / close — "Rope Fence Gate unties", "… ties" (`subtitles-checklist.png`; M5-02).
  - [x] block.skillet.add_food — "Skillet hisses" (`subtitles-checklist.png`).
  - [x] block.skillet.sizzle — "Skillet sizzles": placed skillet on a lit stove (`skillet-sizzle.png`) and held
    skillet cooking beef beside it (`held-subtitles.png`, 3 cooked beef).
  - [x] block.stove.crackle — "Stove crackles" (every screenshot near the stove; M2).
  - [x] block.tomatoes.pick_tomatoes — "Tomatoes pop" (`tomato-crop.png`, ripe vine → age 0).
  - [x] entity.rotten_tomato.throw / hit — "Rotten Tomato flies", "… splatters" (M8-06).
  - [x] item.skillet.attack.strong / weak — recorded on a second client (M2-10).
- particles: `farmersdelight:steam` puffs and bubbles over the heated pot (`pot-steam.png`); `sparkle` over the
  gleaming salad (M7-01); `star` on pet feeding (M4). Ambient non-BE effects stay degraded as planned (E-06).
- world changes: FDM6 pad: stove + cooking pot at 18,100..101,1347, rope gate 16,100,1345, oak cabinet
  15,100,1346; weather cleared, time noon. Game running.
- next: M9-06 (property sets & creative tab).

## M9-06 Property sets & creative tab — PASS — 2026-11-16
- build pin: afe471cf | mod commit: (this commit)
- creative tab: bug fixed in `tools/gen_tab_decl.py` — FD's `BlockItemId id("block", "item")` names the item
  second, the generator took the block id, so the tab listed `onions`, `rice_panicles`, `cabbages`, `tomatoes`
  (blocks, no items) and the engine left them out: Onion, Rice Panicle, Cabbage Seeds and Tomato Seeds were
  missing (since M6-01, which checked only the first page). Regenerated `tools/decl/tabs.toml.in` + decls; every
  one of the 188 ids now names a declared item (script check).
- evidence (FDM6 creative inventory, page 2; `run/shots/m9-06/`): tab "Farmer's Delight" with the stove icon
  (`page2.png`); five scroll positions (`v0`…`v20.png`) read row by row against the reference list: rows 1–5
  (stove … pink canvas sign), row 11 now "colonies, cabbage, tomato, onion, rice panicle, rice, cabbage seeds,
  tomato seeds", row 12 rotten tomato … raw pasta, last row shepherd's pie block … straw bed; 20 full rows + 8 =
  188 = the reference.
- property sets: engine gap **E-37** — `recipe_property_sets` runs inside the data load with a
  `DataLoadContext` that can read neither the load's recipes nor item tags, and the mod's recipe tables are rebuilt
  after it (`tags_updated`), so FD's 9 sets (`cooking_pot_input_one…six`, `cooking_pot_container`,
  `cutting_board_input`, `cutting_board_tool`) cannot be filled from the recipes. Delta **A52**: the 9 sets are not
  provided. FD's only reader is the client's knife-on-board prediction
  (`CuttingBoardBlockEntity#processStoredItemUsingTool` client branch: swing or not); the port's client takes the
  engine's default prediction, the server result is unchanged. `farmersdelight:serving_containers` (M3) stays.
- next: M9-07 (lang).

## M9-07 Lang — PASS — 2026-11-16
- build pin: afe471cf | mod commit: (this commit)
- did: `tools/test_lang.py` — all 44 carried locale files parse; 307 keys the port shows are in `en_us.json`:
  block names of every declared block, item names (`block.` for block items such as the skillet and the debug
  pie, as FD's BlockItems), both effects, every sound subtitle, the tab title, every `*.farmersdelight.*` key
  literal in src/, the advancement texts and the custom tooltips. Coverage of other locales is FD's own (e.g.
  ru_ru 480/509, ja_jp 479/509, de_de 286/509; the rest falls back to en_us as in FD).
- evidence (FDM6, `run/shots/m9-07/`): language set to ru_ru → "Тушёная говядина / Сытость (03:00)", "Собачий
  корм / Если скормить прирученному волку: …" (`ru.png`); ja_jp → "リンゴサイダー / 衝撃吸収 (01:00) / 効果: /
  衝撃吸収レベルの上限 +4", "ビーフシチュー / 栄養 (03:00)" (`ja.png`). Language set back to en_us.
- next: M9-08 (M9 demo).

## M9-08 M9 demo — PASS — 2026-11-16 — **M9 closed**
- build pin: afe471cf | mod commit: (this commit; code as of 494ceb0)
- recorded runs of every M9 Accept item: M9-01 (each config key toggled with the reference effect), M9-02 (book
  tabs/recipes; placement + ghosts on a server client), M9-03 (overlays + client keys), M9-04 (tooltips + key),
  M9-05 (20-sound checklist, particles), M9-06 (creative tab order), M9-07 (lang key script; ru_ru and ja_jp in
  game). Screenshots under `run/shots/m9-0*/`.
- config edits survive restart: M9-01 — `canvasSignDarkBackgroundList ["white"]` written while running, read
  after a restart (white board → white text, red → black); the recipe book's open state also survived a restart
  (M9-02).
- HUD overlays on a guest (dedicated server re-exported with this build, Alice; `run/shots/m9-08/guest-hud.png`):
  Nourishment → gilded drumsticks; Comfort at 0 saturation, 15/20 health → the sheen moving over the hearts across
  four frames. The windowed guest client presented one frame and stalled (window not shown on the desktop);
  relaunched with `--offscreen` it rendered normally — a rig note, not a mod issue.
- **M9 closed.** New in M9: deltas A45–A52, engine gaps E-35 (configs after item defaults), E-36 (single-player
  mod-book placement), E-37 (property sets without recipes); fixed the creative-tab ids (onion, rice panicle,
  seeds). Server and clients stopped; FDM6 game closed; language back to en_us.
- next: M10-01 (PORT_PLAN §11).

## M10-01 Parity sweep — PASS (layer 3) / oracle ENVIRONMENT-BLOCKED — 2026-11-16
- build pin: afe471cf | mod commit: (this commit)
- layer 3 (VALIDATION_PLAN §2.3), all green: (a) `carry_fd.py --check` 2375 carried files equal the manifest;
  (b) `test_fixtures`, `test_food_decls`, `test_knife_decls`, `test_compost_fuel_decls`, `test_gen_decls`,
  `gen_decls --check` (237 files), `gen_worldgen --check`, `gen_conventional_tags --check` (34),
  `gen_food_tooltips --check`, `test_lang` (307 keys, 44 locales), `test_carry`; (c) unit tests parse all 28
  cooking and 117 cutting fixture recipes (`cargo test`: 88 passed); (d) tag membership in game: vanilla tags FD
  extends resolve (`execute if items … #minecraft:enchantable/weapon` with an iron knife → passed;
  `execute if block <stove> #minecraft:mineable/pickaxe` → passed); FD's own and `c:` tags were checked through
  behaviour in their milestones (knives on the board M1/M8, heat sources M2/M3, `#farmersdelight:ropes` with a
  test datapack M9-01).
- fixed while sweeping: `src/items/food_tooltips.rs` is excluded from `cargo fmt` (`#[rustfmt::skip]`), which had
  made its generator check fail.
- engine gap **E-38**: command `#tag` arguments know only vanilla-namespace tags — `#c:tools/knife`,
  `#farmersdelight:heat_sources` (and a world datapack's `#farmersdelight:ropes`) answer "Unknown item/block tag"
  though gameplay resolves them. No mod impact; it only rules out a command-based tag dump.
- layer 4 (Fabric 26.3 + FD reference oracle, §2.4): NOT RUN — ENVIRONMENT-BLOCKED: needs a user-supplied Fabric
  26.3 install with the FD jar (the FD checkout may not be built or run here). Parity rests on source reading,
  fixtures and the in-game checks of every milestone (risk R5).
- triage: no unexplained differences; every known one is an A-delta in this log (A1–A52) with its E gap where
  the engine is the cause.
- next: M10-02 (multiplayer).

## M10-02 Multiplayer — PASS — 2026-11-16
- build pin: afe471cf | mod commit: (this commit; code as of 5faed22)
- VALIDATION_PLAN §3.4 on the dedicated server (re-exported with this build; clients Alice and Bob from the same
  build, both `--offscreen`):
  - single-player actual path: integrated server thread (all milestones ran single player through it; the E-36
    recipe-book gap is the one single-player-only difference).
  - LAN: covered by M2-10/M3/M5-08 (second client sees renders, sounds, screens). `/publish` now answers "Unable
    to host local game" in this offline setup, so the guest checks this time ran on the dedicated server, which
    uses the same hub/guest code.
  - dedicated server: export runs, same-build clients join; a client without the mod (plain developer build,
    "Carol") is refused: "Your mods don't match this server's." (D9).
  - concurrent interaction: beef on one board, Alice and Bob use knives in the same instant → exactly one cut
    (2 Minced Beef, one knife damaged); the second click found an empty board and placed that player's knife on it
    (FD's empty-board placement); no duplication, no loss. Pot menu with two players: M3.
  - multiple players: menu reach and cabinet opener counting with two viewers: M7-09.
  - disconnect/reconnect: client killed mid held-skillet cooking → after reconnect the skillet still holds its
    beef (`farmersdelight:skillet_ingredient` 1) and the off hand 2 (not lost); client killed with 5 tomatoes on
    the cursor in the pot menu → the stack is dropped at the player (vanilla `AbstractContainerMenu#removed` for
    a disconnected player), not lost; the reconnecting player sees the current world state.
- world changes: FDServer test world: board, stove and cabinet near −530,81,70; Alice back in creative; dropped
  tomatoes left at −531,81,68. Server and clients stopped.
- next: M10-03 (persistence).

## M10-03 Persistence — PASS — 2026-11-16
- build pin: afe471cf | mod commit: (this commit; code as of 5faed22)
- VALIDATION_PLAN §3.3 for the six block entities on the FDM6 pad (stove 12,100,1340 with beef; skillet on a lit
  stove 14,101,1340 with beef; cooking pot on a stove 18,101,1347 with beef/carrot/potato → beef stew; cutting board
  17,100,1343 with chicken; wooden basket 10,100,1340 with 7 carrots; oak cabinet 15,100,1346 with 5 apples):
  - partial cooking without heat: the pot's stove set unlit → `CookTime` 178 → 132 in about a second (−2/tick);
    relit, it climbs again. (Heat-source changes and hopper conductors: M2/M3.)
  - chunk unload/reload: snapshot, `tp` 4000 blocks away for 10 s (spectator), back: stove progress +66 (the ~3 s
    after returning, not the 10 s away), skillet and pot continued their cycles, board/basket/cabinet contents
    identical; no double cooking.
  - restart: `tick freeze`, snapshot, `ctl quit`, relaunch: skillet 38 → 52, pot 41 → 56 (≈15 ticks of normal
    progress before the snapshot), pot ingredients identical, stove totals identical, board/basket/cabinet
    identical; nothing cooked at once on load (the 3.6.30 "cooked immediately on reload" bug absent). Item
    components across restart: carved board and skillet ingredient (M8-08, M10-02).
- note: a finished stove slot keeps its progress number until a new item goes in (FD's `addItem` resets it), as
  in the reference.
- world changes: FDM6 pad 10..15,99..101,1338..1341 (stoves, skillet, basket). Game running.
- next: M10-04 (performance).

## M10-04 Performance — PASS — 2026-11-16
- build pin: afe471cf | mod commit: (this commit; code as of 5faed22)
- method: FDM6 single player (server thread), player parked at the test field (20,110,1340); `/tick query`
  (100-tick samples) three times 6 s apart per phase, read in one session for an A/B against the same world state.
  The world's own load moved between ~4 ms (first sample) and ~10–11 ms (later, rain + nearby village/entities), so
  each phase is compared with a baseline taken next to it.
- results (average ms/tick; P50/P95/P99 in the log lines of this session):
  - no FD block entities in the field: 10.4 / 10.5 / 8.1; later 11.5 / 10.8 / 10.1.
  - 200 idle cooking pots (no heat, `fill`): 11.7 / 11.1 / 10.3 → ≈ +0.6 ms (target < 2 ms idle: met).
  - 200 lit stoves each cooking 3 raw meats (progress 42 → 529 during the samples): 8.2 / 11.5 / 10.3 → no
    difference beyond noise (target < 5 ms active: met).
- incidental: stoves filled with item NBT but no `CookingTimes` totals cook everything on the first tick and pop
  it (604 item entities, +7 ms from the items); FD reads the same NBT the same way (totals default to 0). Not a
  gameplay path (players' items get their totals on insertion).
- world changes: test field cleared (pots, stoves and dropped items removed). Game running.
- next: M10-05 (wasm probe, non-blocking).

## M10-05 Wasm probe (non-blocking) — build PASS / run NOT RUN — 2026-11-16
- build pin: afe471cf | mod commit: (this commit; code as of c64b2f6)
- `cargo horizon-mod build --wasm` on the repo refuses ("native mod in wasm group": a mod carries native or wasm
  code, not both). On a scratch copy (`git archive HEAD`) with the `[native]` table removed it builds in 1 m 41 s
  with toolchain 1.99.1: `mod.wasm` 9.2 MB, `target/horizon-mod/ported_farmersdelight.hmod` 4.7 MB, `[wasm]` sha512
  written. So the code stays within the wasm-compatible API subset (D3).
- not run: loading the wasm package in a game and repeating the M1 script (needs a separate wasm instance; no
  release gate).
- next: M10-06 (release packaging).

## M10-06 Release packaging — PASS — 2026-11-16
- build pin: afe471cf | mod commit: (this commit)
- version `26.3-3.6.30-horizon.1` (horizon-mod.toml); README rewritten (features, install, multiplayer rule D9,
  player-visible differences with their A/E ids, build checks), CHANGELOG.md added.
- strict checks: `cargo fmt --all --check` clean; `cargo clippy --all-targets --locked -- -D warnings` clean after
  two fixes (knife.rs late init; generated tooltip table now uses an `Effect` type alias); `cargo test --locked`
  88 passed; all generator/test scripts green.
- package: `cargo horizon-mod build` → `target/horizon-mod/ported_farmersdelight.hmod` (2.8 MB, SHA-256
  2f1b7ba6…6f13); `cargo horizon-mod check` clean ("native-mod, format native-1/afe471cf…/e59da909…, for 26.3");
  contains horizon-mod.toml, LICENSE, LICENSE-farmersdelight, NOTICE, README.md (third_party/ is not packaged; its
  licence text ships as LICENSE-farmersdelight, M0 decision).
- fresh-instance install: `instance create fd-fresh`; the .hmod copied into `horizon-native-mods/`;
  `native-build --instance fd-fresh` built it; a new world `FDFresh` started (offscreen): FD stove given, cooking
  recipe unlocked, a cooking pot placed, both config files written. Dedicated server export: M10-02.
- next: M10-07 (published-jar recheck).

## M10-07 Published-jar recheck — PASS — 2026-11-16 — **M10 closed**
- build pin: afe471cf | mod commit: (this commit)
- the published `FarmersDelight-26.3-3.6.30+refabricated.jar` (Modrinth, sha512 099f7245… as recorded in M0-07)
  downloaded to an isolated scratch folder and read as a zip only (never run). Every one of the 2375 carried files
  (`tools/carry_manifest.json` sources, `src/main/resources/` and `src/generated/resources/`) is byte-identical
  (SHA-256) to the jar's entry at the same path: 2375 same, 0 differ, 0 missing.
- the jar's 86 data/asset files that are not carried are all outside the core scope (D8): integration recipes
  (Create 11, Immersive Engineering 16, Silent Gear 1), recipe-viewer files (JEI/EMI/EIV textures, `rrv`, `emi`),
  CraftTweaker scripts, Create/Create Addition/Serene Seasons/Sable/Origins/TConstruct tags and Create data,
  `weapon_attributes/skillet.json` (Better Combat). Triaged: nothing to carry.
- **M10 closed; the port's plan (M0–M10) is complete.** Open items stay recorded: Fabric oracle (M10-01, needs a
  user-supplied Fabric+FD install), wasm run (M10-05, non-blocking), engine gaps E-01…E-38 with their deltas.

# Add-on base extensions (addons/ADDONS_PLAN.md §6; user approved all BX tasks on 2026-11-16)

## BX-07 Release and API-version contract — PASS — 2026-11-16
- build pin: afe471cf | mod commit: (this commit)
- found: the M10-06 version `26.3-3.6.30-horizon.1` is not semver (Horizon `parse_version` →
  `semver::Version::parse` fails on `26.3-…`), so no mod could depend on the base. Version is now `3.6.30-horizon.2`
  (AD13 proposal; a pre-release tag, so add-ons pin it with `=`).
- `[properties] fd_port_api = 1`; `pub mod api` with `API_VERSION = 1` (unit test: property = constant);
  README "For add-on authors" (exact pin, what is stable, bump rule); CHANGELOG entry.
- in game (fd-port instance, world FDM6, offscreen): a scratch content mod `fdaddon_probe` with
  `ported_farmersdelight = "=3.6.30-horizon.2"` loads ("content from 2 mod(s)"); with `=3.6.30-horizon.1` startup is
  refused: "fdaddon_probe requires ported_farmersdelight =3.6.30-horizon.1, but installed version is
  3.6.30-horizon.2" (the client reports it as a crash with a crash report, the engine's behaviour). The probe was
  removed from the instance afterwards. Worlds saved earlier log "mod ported_farmersdelight was 0.1.0 when the world
  was saved" once (warning only).
- evidence: `run/shots/bx-07/` (accepted.log, refusal.log, crash report).
- next: BX-08 (base carry library).

## BX-08 Base carry library — PASS — 2026-11-16
- build pin: afe471cf | mod commit: (this commit)
- `tools/carry_lib.py`: the FD carry's reusable parts as functions (canonical JSON writer `dump_json`,
  `fabric:load_conditions` rewrite, `[[rewrite]]` rules, multi-root `collect` with the overlap check, manifest
  read/write/stale removal, `git_head`), plus new add-on helpers: `fence_violations` (package namespace fence, owned
  namespaces + `data/{minecraft,c}/tags/**`) and the tag universe (`tags_in_tree`, `tags_in_zip`, `tag_references`,
  `undefined_tags`).
- `carry_fd.py` now uses it; `carry_fd.py --check` → "carry check ok: 2375 files" (manifest byte-identical, so the
  output is unchanged); `test_carry.py` 7 OK; new `test_carry_lib.py` 4 OK (fence, tag definitions from a tree and a
  jar, references/report, writer format). Add-on carries will import it with `--base <checkout>` and compare
  `git_head` with their pinned base commit (MD-02).
- next: BX-01 (convention tags for MD/CD).

## BX-01 Ship Fabric API convention tags — PASS — 2026-11-16
- build pin: afe471cf | mod commit: (this commit)
- second carry source `[fapi]` in `tools/carry_rules.toml.in`: FabricMC/fabric tag `0.160.4+26.3` @ `542047ff…`,
  `fabric-convention-tags-v2/src/generated/resources/data/c/tags/**` (sparse clone `sources/fabric-api-conventions`,
  now also `/LICENSE`); `carry_fd.py --fapi <checkout>` (pin checked like FD's). Full set: 523 files (item 270,
  block 132, worldgen 95, fluid 12, enchantment 7, entity_type 6, potion 1).
- found: FD carries 28 `c` tag files Fabric API also defines; on Fabric both load and merge, here only FD's shipped,
  so e.g. `c:drinks/milk` lacked `minecraft:milk_bucket`, `c:foods/raw_meat` lacked vanilla raw meats and rabbit,
  `c:foods/vegetable` lacked carrot/potato/beetroot (a parity gap since M0; no delta had recorded it). Now such a
  file is written once: FD's entries, then Fabric's missing ones (rule `merge_fapi`, 24 files changed, 4 already
  equal). The other 495 are byte copies.
- `tools/gen_conventional_tags.py` and its two tables retired: all 34 files it wrote are in the Fabric set (values
  equal, except `c:mushrooms`, which Fabric writes as `#minecraft:mushrooms`, a vanilla tag with the same members).
- licence: `LICENSE-fabric-api` (packaged) + `third_party/fabric-api/LICENSE`, NOTICE section, README; CI clones the
  sparse Fabric checkout and runs `carry_fd.py --fapi … --check` and `test_carry_lib.py`.
- new `tools/tag_report.py` (carry_lib tag universe; optional `required: false` references skipped): base alone
  211 referenced tags, 0 undefined; base + MD + CD sources, `c` only: 158 referenced, undefined only CD's
  `c:foods/milk` (5 files, pruned by CD-03, CA5) and Expanded Delight integration files (excluded, AD8).
- in game (fd-port, world FDM6, offscreen, pad at 60000,200,60002 removed afterwards): three pots on lit campfires
  filled with `data merge`: hot cocoa from a **milk bucket** (`#c:drinks/milk`) cooked, the empty bucket was
  ejected; dog food from a **raw rabbit** (`#c:foods/raw_meat`) cooked; hot cocoa from FD's milk bottle still cooks.
  Startup log: no new recipe errors (the 3 crate-recipe "unknown load conditions" lines are the known M0 client-view
  message). Wheat/bread `c` tags were already shipped by the retired generator (unchanged in effect).
- carry check ok: 2870 files; cargo test 89; `.hmod` built and checked.
- evidence: `run/shots/bx-01/` (tag reports, screenshot).
- next: BX-02 (add-on tag overlay for FD-namespace tags).
