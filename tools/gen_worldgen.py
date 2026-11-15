#!/usr/bin/env python3
"""Render FD's code-side worldgen registrations as data files Horizon reads.

- tools/worldgen/biome_modifiers.toml.in → data/farmersdelight/neoforge/biome_modifier/<NN>_<feature>.json
  (NeoForge-format biome modifiers; NN = FD's registration order).
- tools/worldgen/villages.toml.in → data/farmersdelight/worldgen/template_pool/village/<pool>.json and
  worldgen/processor_list/<list>.json (`horizon:add_to` additions to vanilla's village pools and farm lists).

Each output folder holds only generated files: others there are removed. `--check` verifies them.
"""
import argparse
import json
import sys
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
TABLES = REPO / "tools" / "worldgen"
DATA = REPO / "data" / "farmersdelight"
FOLDERS = {
    "biome_modifier": DATA / "neoforge" / "biome_modifier",
    "pool": DATA / "worldgen" / "template_pool" / "village",
    "processor_list": DATA / "worldgen" / "processor_list",
}


def load(name: str) -> dict:
    with (TABLES / name).open("rb") as f:
        return tomllib.load(f)


def dump(body: dict) -> bytes:
    return (json.dumps(body, indent=2) + "\n").encode()


def biomes(m: dict):
    terms = [f"#{m['allowed']}"]
    if "denied" in m:
        terms.append({"type": "neoforge:not", "value": f"#{m['denied']}"})
    if "temperature" in m:
        lo, hi = m["temperature"]
        terms.append({"type": "horizon:climate", "temperature": {"min": lo, "max": hi}})
    return terms[0] if len(terms) == 1 else {"type": "neoforge:and", "values": terms}


def render() -> dict[str, dict[str, bytes]]:
    out: dict[str, dict[str, bytes]] = {k: {} for k in FOLDERS}
    for i, m in enumerate(load("biome_modifiers.toml.in")["modifier"]):
        out["biome_modifier"][f"{i:02d}_{m['feature']}.json"] = dump(
            {
                "type": "neoforge:add_features",
                "biomes": biomes(m),
                "features": f"farmersdelight:{m['feature']}",
                "step": "vegetal_decoration",
            }
        )
    villages = load("villages.toml.in")
    for h in villages["house"]:
        name = h["pool"].removeprefix("minecraft:village/").replace("/", "_")
        out["pool"][f"{name}.json"] = dump(
            {
                "horizon:add_to": h["pool"],
                "elements": [
                    {
                        "weight": h["weight"],
                        "element": {
                            "element_type": "minecraft:single_pool_element",
                            "location": h["piece"],
                            "processors": "minecraft:empty",
                            "projection": "rigid",
                        },
                    }
                ],
            }
        )
    for farm in villages["farm"]:
        rules = [
            {
                "input_predicate": {"predicate_type": "minecraft:random_block_match", "block": block, "probability": p},
                "location_predicate": {"predicate_type": "minecraft:always_true"},
                "output_state": {"id": crop},
            }
            for block, p, crop in villages["crops"][farm["crops"]]
        ]
        out["processor_list"][f"{farm['list'].removeprefix('minecraft:')}.json"] = dump(
            {"horizon:add_to": farm["list"], "processors": [{"processor_type": "minecraft:rule", "rules": rules}]}
        )
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    bad, written = [], 0
    for kind, files in render().items():
        folder = FOLDERS[kind]
        present = {p.name for p in folder.glob("*.json")} if folder.is_dir() else set()
        if args.check:
            bad += [f"{kind}/{n}" for n in files if not (folder / n).exists() or (folder / n).read_bytes() != files[n]]
            bad += [f"stale {kind}/{n}" for n in present - set(files)]
            continue
        folder.mkdir(parents=True, exist_ok=True)
        for name in present - set(files):
            (folder / name).unlink()
        for name, body in files.items():
            (folder / name).write_bytes(body)
        written += len(files)
    if args.check:
        for b in bad:
            print(f"out of date: {b}", file=sys.stderr)
        if not bad:
            print("worldgen check ok")
        return 1 if bad else 0
    print(f"wrote {written} worldgen file(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
