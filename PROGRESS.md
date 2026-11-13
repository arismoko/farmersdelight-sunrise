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
