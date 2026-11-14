#!/usr/bin/env python3
"""M4-01: every declared food's nutrition/saturation/always-edible equals the FoodValues constant its
decl row cites (tests/fixtures/foods.json, extracted from FD's Java), and every bowl/bottle food
returns its container and stacks to 16."""
import json
import re
import struct
import tomllib
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
FOODS = {r["constant"]: r for r in json.loads((REPO / "tests/fixtures/foods.json").read_text())["foods"]}
ROWS = tomllib.loads((REPO / "tools/decl/items.toml.in").read_text())


def f32(x: float) -> float:
    return struct.unpack("f", struct.pack("f", x))[0]


class FoodDeclsTest(unittest.TestCase):
    def test_food_values_match_the_reference(self):
        checked = 0
        for name, row in ROWS.items():
            m = re.search(r"FoodValues\.java:([A-Z_]+)", row["source"])
            food = row.get("components", {}).get("minecraft:food")
            if not m:
                self.assertIsNone(food, name)
                continue
            ref = FOODS[m.group(1)]
            self.assertEqual(food["nutrition"], ref["nutrition"], name)
            want = f32(ref["nutrition"] * ref["saturation_modifier"] * 2.0)
            self.assertEqual(f32(food["saturation"]), want, name)
            self.assertEqual(food.get("can_always_eat", False), ref["always_edible"], name)
            self.assertIn("minecraft:consumable", row["components"], name)
            checked += 1
        self.assertEqual(checked, 77)  # every food item but the three non-food drinks (milk, cocoa, juice)

    def test_containers_and_stacks(self):
        for name, row in ROWS.items():
            remainder = row.get("crafting_remainder")
            if remainder in ("minecraft:bowl", "minecraft:glass_bottle"):
                use = row["components"].get("minecraft:use_remainder", {}).get("id")
                self.assertEqual(use, remainder, name)
                if name != "tomato_sauce":  # TOMATO_SAUCE: foodItem + craftRemainder, stack 64
                    self.assertEqual(row.get("max_stack_size"), 16, name)

    def test_q3_onion_soup_uses_shepherds_pie_values(self):
        self.assertIn("FoodValues.java:SHEPHERDS_PIE", ROWS["onion_soup"]["source"])


if __name__ == "__main__":
    unittest.main()
