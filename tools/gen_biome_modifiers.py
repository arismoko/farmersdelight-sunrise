#!/usr/bin/env python3
"""Render FD's biome selection (tools/worldgen/biome_modifiers.toml.in) as NeoForge-format biome modifiers.

Writes data/farmersdelight/neoforge/biome_modifier/<NN>_<feature>.json (NN = FD's registration order) and removes
other files there. `--check` verifies the folder matches.
"""
import argparse
import json
import sys
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SRC = REPO / "tools" / "worldgen" / "biome_modifiers.toml.in"
OUT = REPO / "data" / "farmersdelight" / "neoforge" / "biome_modifier"


def biomes(m: dict):
    terms = [f"#{m['allowed']}"]
    if "denied" in m:
        terms.append({"type": "neoforge:not", "value": f"#{m['denied']}"})
    if "temperature" in m:
        lo, hi = m["temperature"]
        terms.append({"type": "horizon:climate", "temperature": {"min": lo, "max": hi}})
    return terms[0] if len(terms) == 1 else {"type": "neoforge:and", "values": terms}


def render() -> dict[str, bytes]:
    with SRC.open("rb") as f:
        mods = tomllib.load(f)["modifier"]
    out = {}
    for i, m in enumerate(mods):
        body = {
            "type": "neoforge:add_features",
            "biomes": biomes(m),
            "features": f"farmersdelight:{m['feature']}",
            "step": "vegetal_decoration",
        }
        out[f"{i:02d}_{m['feature']}.json"] = (json.dumps(body, indent=2) + "\n").encode()
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    files = render()
    present = {p.name for p in OUT.glob("*.json")} if OUT.is_dir() else set()
    if args.check:
        bad = sorted(n for n in files if not (OUT / n).exists() or (OUT / n).read_bytes() != files[n])
        bad += sorted(f"stale {n}" for n in present - set(files))
        for b in bad:
            print(f"out of date: {b}", file=sys.stderr)
        if not bad:
            print(f"biome modifier check ok: {len(files)} files")
        return 1 if bad else 0
    OUT.mkdir(parents=True, exist_ok=True)
    for name in present - set(files):
        (OUT / name).unlink()
    for name, body in files.items():
        (OUT / name).write_bytes(body)
    print(f"wrote {len(files)} biome modifier(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
