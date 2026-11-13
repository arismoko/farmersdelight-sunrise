#!/usr/bin/env python3
"""Count checks on the committed parity fixtures (PORT_PLAN M0-09 acceptance)."""
import json
import unittest
from pathlib import Path

FIX = Path(__file__).resolve().parent.parent / "tests" / "fixtures"


def load(name):
    return json.loads((FIX / f"{name}.json").read_text("utf-8"))[name]


class FixtureCounts(unittest.TestCase):
    def test_recipes(self):
        self.assertEqual(len(load("cutting_recipes")), 117)
        self.assertEqual(len(load("cooking_recipes")), 28)

    def test_foods_and_knives(self):
        foods = {f["constant"]: f for f in load("foods")}
        self.assertEqual(len(foods), 73)
        self.assertEqual((foods["BEEF_PATTY"]["nutrition"], foods["BEEF_PATTY"]["saturation"]), (4, 6.4))
        self.assertEqual(len(load("knives")), 6)
        self.assertEqual(load("knives")["flint_knife"]["durability"], 131)

    def test_config(self):
        cfg = load("config_defaults")
        self.assertEqual((len(cfg["common"]), len(cfg["client"])), (19, 3))

    def test_beef_cutting(self):
        beef = next(r for r in load("cutting_recipes") if r["id"] == "farmersdelight:cutting/beef")
        self.assertEqual(beef["json"]["result"][0]["item"], {"count": 2, "id": "farmersdelight:minced_beef"})


if __name__ == "__main__":
    unittest.main()
