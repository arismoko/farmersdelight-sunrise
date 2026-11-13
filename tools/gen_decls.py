#!/usr/bin/env python3
"""Generate Horizon content declarations from tools/decl/*.toml (PORT_PLAN M0-06).

Each tools/decl/<kind>.toml holds one table per id (the path under `farmersdelight:`). Every row
carries `source = "<FD file:SYMBOL>"` (dropped from the output); the other keys are the
declaration exactly as Horizon reads it. Keys are checked against the build pin's
`deny_unknown_fields` structs (crates/horizon-mod-package/src/content.rs @ afe471cf) so a typo fails
here rather than at game load.

Usage: gen_decls.py [--check]
"""
from __future__ import annotations

import argparse
import json
import sys
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
DECL = REPO / "tools" / "decl"
OUT = REPO / "data" / "farmersdelight" / "horizon"
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
    where = f"tools/decl/{kind}.toml [{name}]"
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
    """Relative output path (under data/farmersdelight/horizon) -> file bytes."""
    out: dict[str, bytes] = {}
    for kind in sorted(KINDS):
        path = decl / f"{kind}.toml"
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
            print(f"out of date: data/{NAMESPACE}/horizon/{r}", file=sys.stderr)
        if bad or stale:
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
    print(f"wrote {len(files)} declaration(s), removed {len(stale)} stale")
    return 0


if __name__ == "__main__":
    sys.exit(main())
