#!/usr/bin/env python3
"""Generate engine content declarations from tools/decl/*.toml.in (PORT_PLAN M0-06).

Each tools/decl/<kind>.toml.in holds one table per id (the path under `farmersdelight:`). Every row
carries `source = "<FD file:SYMBOL>"` (dropped from the output); the other keys are the
declaration exactly as the engine reads it. Keys are checked against the build pin's
`deny_unknown_fields` structs (crates/sunrise-mod-package/src/content.rs @ 2d81263) so a typo fails
here rather than at game load. The output dir comes from tools/target.toml.in (`decl_dir`), and the
generator also writes src/engine.rs (the engine's macro names); `--check` covers both and fails on
declarations left under the other engine's dir.

Usage: gen_decls.py [--check]
"""
from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import target  # noqa: E402

REPO = Path(__file__).resolve().parent.parent
DECL = REPO / "tools" / "decl"
OUT = REPO / "data" / "farmersdelight" / target.DECL_DIR
ENGINE_RS = REPO / "src" / "engine.rs"
NAMESPACE = "farmersdelight"

BLOCK_PROPERTIES = {
    "hardness", "explosion_resistance", "light", "dynamic_light", "requires_tool", "friction",
    "map_color", "sounds", "random_ticks", "render_shape", "push_reaction", "occludes",
    "destroy_overlay",
}
KINDS = {
    # decl file stem -> (output dir, allowed top-level keys, required keys)
    "blocks": ("block", {
        "template", "properties", "shapes", "collision_shapes", "outline_context_shapes",
        "collision_context_shapes", "interaction_shapes", "state_light", "state_properties",
        "default_state", "connects_to", "thickness", "wood_type", "wall_block", "sign_text_color",
        "sign_text_glow", "base_block", "item", "open_sound", "close_sound", "block_item",
        "pick_item", "components",
    }, {"template"}),
    "items": ("item", {
        "template", "max_stack_size", "rarity", "components", "crafting_remainder", "block",
        "arm_pose",
    }, {"template"}),
    "effects": ("mob_effect", {"category", "color", "sort_order"}, {"category", "color"}),
    "entities": ("entity_type", {
        "template", "item", "damage", "attributes", "passenger_attachments", "sounds",
    }, {"template"}),
    "menus": ("menu", {"slots", "data_slots", "results", "crafting", "links"}, {"slots"}),
    "tabs": ("creative_tab", {"title", "icon", "items"}, {"title", "icon", "items"}),
}
META = {"source"}


class DeclError(Exception):
    pass


def validate(kind: str, name: str, row: dict) -> dict:
    _, allowed, required = KINDS[kind]
    where = f"tools/decl/{kind}.toml.in [{name}]"
    if not name or any(not (c.islower() or c.isdigit() or c in "_/.-") for c in name):
        raise DeclError(f"{where}: bad id path")
    if "source" not in row:
        raise DeclError(f"{where}: missing source citation")
    unknown = set(row) - allowed - META
    if unknown:
        raise DeclError(f"{where}: unknown key(s) {sorted(unknown)}")
    missing = required - set(row)
    if missing:
        raise DeclError(f"{where}: missing key(s) {sorted(missing)}")
    if kind == "blocks" and "properties" in row:
        bad = set(row["properties"]) - BLOCK_PROPERTIES
        if bad:
            raise DeclError(f"{where}: unknown properties key(s) {sorted(bad)}")
    return {k: v for k, v in row.items() if k not in META}


def render(decl: Path = DECL) -> dict[str, bytes]:
    """Relative output path (under data/farmersdelight/<decl_dir>) -> file bytes."""
    out: dict[str, bytes] = {}
    for kind in sorted(KINDS):
        path = decl / f"{kind}.toml.in"
        if not path.exists():
            continue
        with path.open("rb") as f:
            rows = tomllib.load(f)
        for name, row in rows.items():
            if not isinstance(row, dict):
                raise DeclError(f"{path.name}: top-level key {name!r} is not a table")
            body = validate(kind, name, row)
            rel = f"{KINDS[kind][0]}/{name}.json"
            out[rel] = (json.dumps(body, indent=2, ensure_ascii=False) + "\n").encode("utf-8")
    return out


def existing(root: Path = OUT) -> set[str]:
    return {p.relative_to(root).as_posix() for p in root.rglob("*.json")} if root.exists() else set()


def foreign_decls() -> list[str]:
    """Committed files under data/*/<engine>/ for an engine other than the target's."""
    out = []
    for other in target.ENGINES:
        if other == target.DECL_DIR:
            continue
        for p in (REPO / "data").glob(f"*/{other}"):
            out += [q.relative_to(REPO).as_posix() for q in p.rglob("*") if q.is_file()]
    return sorted(out)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--check", action="store_true", help="fail if generated files are out of date")
    args = ap.parse_args(argv)
    try:
        files = render()
    except (DeclError, tomllib.TOMLDecodeError) as e:
        print(f"gen_decls FAILED: {e}", file=sys.stderr)
        return 1
    stale = sorted(existing() - set(files))
    if args.check:
        bad = [r for r, b in files.items() if not (OUT / r).exists() or (OUT / r).read_bytes() != b]
        for r in bad + stale:
            print(f"out of date: data/{NAMESPACE}/{target.DECL_DIR}/{r}", file=sys.stderr)
        foreign = foreign_decls()
        for r in foreign:
            print(f"declaration under another engine's dir: {r}", file=sys.stderr)
        engine_bad = not ENGINE_RS.exists() or ENGINE_RS.read_bytes() != target.engine_rs()
        if engine_bad:
            print("out of date: src/engine.rs", file=sys.stderr)
        if bad or stale or foreign or engine_bad:
            return 1
        print(f"decl check ok: {len(files)} files")
        return 0
    for rel, body in files.items():
        p = OUT / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        if not p.exists() or p.read_bytes() != body:
            p.write_bytes(body)
    for rel in stale:
        (OUT / rel).unlink()
    if not ENGINE_RS.exists() or ENGINE_RS.read_bytes() != target.engine_rs():
        ENGINE_RS.write_bytes(target.engine_rs())
    print(f"wrote {len(files)} declaration(s), removed {len(stale)} stale")
    return 0


if __name__ == "__main__":
    sys.exit(main())
