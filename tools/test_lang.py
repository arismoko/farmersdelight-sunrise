#!/usr/bin/env python3
"""M9-07: every translation key the port shows is in en_us.json, and every locale file is valid JSON.

Keys checked: `item.`/`block.farmersdelight.<id>` for each declared item and block (items whose block is not
declared as an item use `block.` like vanilla BlockItems), the effects, the sound subtitles, the creative tab
title, every `farmersdelight` key written as a string literal in src/, and every translate key in the carried
advancements.
"""
import glob
import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LANG = ROOT / "assets/farmersdelight/lang"


def main() -> int:
    en = json.loads((LANG / "en_us.json").read_text())
    for f in sorted(LANG.glob("*.json")):
        json.loads(f.read_text())
    blocks = tomllib.loads((ROOT / "tools/decl/blocks.toml.in").read_text())
    items = tomllib.loads((ROOT / "tools/decl/items.toml.in").read_text())
    needed = set()
    for b in blocks:
        needed.add(f"block.farmersdelight.{b}")
    for i, decl in items.items():
        # A BlockItem is named by its block (`block.farmersdelight.<block>`), as the skillet and the debug pie are.
        block = decl.get("block", f"farmersdelight:{i}" if i in blocks else None)
        if block:
            needed.add("block." + block.replace(":", "."))
        else:
            needed.add(f"item.farmersdelight.{i}")
    for e in ("nourishment", "comfort"):
        needed.add(f"effect.farmersdelight.{e}")
    for s, d in json.loads((ROOT / "assets/farmersdelight/sounds.json").read_text()).items():
        if "subtitle" in d:
            needed.add(d["subtitle"])
    needed.add("itemGroup.farmersdelight")
    for f in glob.glob(str(ROOT / "src/**/*.rs"), recursive=True):
        for k in re.findall(r'"((?:tooltip|container|farmersdelight|block|item|effect)\.farmersdelight\.[a-z0-9_.]+)"',
                            Path(f).read_text()):
            needed.add(k)
    for f in glob.glob(str(ROOT / "data/farmersdelight/advancement/**/*.json"), recursive=True):
        for k in re.findall(r'"translate":\s*"([^"]+)"', Path(f).read_text()):
            needed.add(k)
    for path in ("milk_bottle", "hot_cocoa", "melon_juice"):
        needed.add(f"tooltip.farmersdelight.{path}")
    missing = sorted(k for k in needed if k not in en)
    print(f"{len(needed)} keys checked, {len(list(LANG.glob('*.json')))} locales")
    if missing:
        print("missing in en_us:", *missing, sep="\n  ")
        return 1
    print("lang check ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
