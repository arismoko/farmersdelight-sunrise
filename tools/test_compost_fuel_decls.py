#!/usr/bin/env python3
"""M4-07: every declared item or block in tests/fixtures/compostables.json carries the composter tier of its
chance, no other row has one, and the fuel items burn as FD registers them. Re-run at every milestone:
ids declared later (crops, baskets, straw bed) must carry their value in their own decl row."""
import json
import tomllib
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
CHANCES = json.loads((REPO / "tests/fixtures/compostables.json").read_text())["compostables"]
TIER = {1.0: "always_add_one", 0.85: "medium_high", 0.65: "medium", 0.5: "low_medium", 0.3: "low"}
FUELS = {"straw": 100, "canvas": 400, "tree_bark": 200, "cutting_board": 200}


def rows():
    for kind in ("items", "blocks"):
        for name, row in tomllib.loads((REPO / f"tools/decl/{kind}.toml.in").read_text()).items():
            if kind == "blocks" and row.get("item") is False:
                continue  # no item of its own: its item (if any) is a separate items row
            yield name, row.get("components", {})


class CompostFuelTest(unittest.TestCase):
    def test_compost_tiers(self):
        for name, comps in rows():
            want = CHANCES.get(f"farmersdelight:{name}")
            got = comps.get("minecraft:compostable", {}).get("layers")
            self.assertEqual(got, want and f"minecraft:compostable/{TIER[want]}", name)

    def test_fuels(self):
        burn = {name: c["minecraft:cooking_fuel"]["burn_time"] for name, c in rows() if "minecraft:cooking_fuel" in c}
        for name, ticks in FUELS.items():
            self.assertEqual(burn.get(name), ticks, name)


if __name__ == "__main__":
    unittest.main()
