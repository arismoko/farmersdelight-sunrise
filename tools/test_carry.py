#!/usr/bin/env python3
"""Unit tests for tools/carry_fd.py on a tiny fixture tree."""
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import carry_fd  # noqa: E402

RULES = {k: v for k, v in carry_fd.load_rules(carry_fd.RULES).items() if k not in ("overlay", "base_tag")}


def write(root: Path, rel: str, text: str) -> None:
    p = root / rel
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text, "utf-8")


class CarryTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.fd = Path(self.tmp.name) / "fd"
        self.out = Path(self.tmp.name) / "out"
        gen = "src/generated/resources/"
        main = "src/main/resources/"
        write(self.fd, gen + "data/farmersdelight/recipe/a.json", '{"type":"x"}')
        write(
            self.fd,
            gen + "data/farmersdelight/recipe/crate.json",
            '{"fabric:load_conditions":[{"condition":"farmersdelight:vanilla_crates_enabled"}],"type":"y"}',
        )
        write(self.fd, gen + "data/sereneseasons/tags/item/x.json", "{}")
        write(self.fd, main + "data/farmersdelight/recipe/integration/create/z.json", "{}")
        write(self.fd, main + "assets/farmersdelight/textures/gui/jei/a.png", "png")
        write(self.fd, main + "assets/farmersdelight/lang/en_us.json", '{ "k" : "v" }')
        write(self.fd, main + "fabric.mod.json", "{}")

    def tearDown(self):
        self.tmp.cleanup()

    def test_selection_copy_and_rewrite(self):
        m = carry_fd.carry(self.fd, self.out, RULES)
        dests = [e["dest"] for e in m["files"]]
        self.assertEqual(
            dests,
            [
                "assets/farmersdelight/lang/en_us.json",
                "data/farmersdelight/recipe/a.json",
                "data/farmersdelight/recipe/crate.json",
            ],
        )
        # untouched files are byte copies (no reformatting)
        self.assertEqual((self.out / dests[0]).read_text("utf-8"), '{ "k" : "v" }')
        crate = json.loads((self.out / dests[2]).read_text("utf-8"))
        self.assertEqual(list(crate), ["neoforge:conditions", "type"])
        self.assertEqual(crate["neoforge:conditions"], [{"type": "ported_farmersdelight:vanilla_crates_enabled"}])
        self.assertEqual([e["rule"] for e in m["files"]].count("load_conditions"), 1)

    def test_deterministic(self):
        a = carry_fd.carry(self.fd, self.out, RULES)
        b = carry_fd.carry(self.fd, self.out, RULES)
        self.assertEqual(a, b)

    def test_unknown_condition_fails(self):
        write(
            self.fd,
            "src/generated/resources/data/farmersdelight/recipe/bad.json",
            '{"fabric:load_conditions":[{"condition":"fabric:all_mods_loaded"}]}',
        )
        with self.assertRaises(carry_fd.CarryError):
            carry_fd.carry(self.fd, self.out, RULES)

    def test_overlap_fails(self):
        write(self.fd, "src/main/resources/data/farmersdelight/recipe/a.json", "{}")
        with self.assertRaises(carry_fd.CarryError):
            carry_fd.carry(self.fd, self.out, RULES)

    def test_stale_removed(self):
        old = carry_fd.carry(self.fd, self.out, RULES)
        (self.fd / "src/generated/resources/data/farmersdelight/recipe/a.json").unlink()
        new = carry_fd.carry(self.fd, self.out, RULES)
        stale = carry_fd.remove_stale(self.out, old, new)
        self.assertEqual(stale, ["data/farmersdelight/recipe/a.json"])
        self.assertFalse((self.out / stale[0]).exists())

    def test_villager_trade_tag_entries_flattened(self):
        write(
            self.fd,
            "src/generated/resources/data/minecraft/tags/villager_trade/farmer/level_1.json",
            '{"values":[{"id":"farmersdelight:farmer/1/onion_emerald","required":false},"#minecraft:x"]}',
        )
        carry_fd.carry(self.fd, self.out, RULES)
        tag = json.loads(
            (self.out / "data/minecraft/tags/villager_trade/farmer/level_1.json").read_text("utf-8")
        )
        self.assertEqual(tag["values"], ["farmersdelight:farmer/1/onion_emerald", "#minecraft:x"])

    def test_worldgen_rewrites(self):
        gen = "src/main/resources/data/farmersdelight/worldgen/"
        write(
            self.fd,
            gen + "feature/patch.json",
            '{"type":"farmersdelight:in_order","features":[{"feature":{"type":"minecraft:simple_block"},'
            '"placement":[{"type":"minecraft:offset"}]}]}',
        )
        write(
            self.fd,
            gen + "placed_feature/patch.json",
            '{"feature":"farmersdelight:patch","placement":[{"type":"minecraft:biome"},'
            '{"type":"farmersdelight:biome_tag","tag":"minecraft:is_overworld"},{"type":"minecraft:offset"}]}',
        )
        write(self.fd, gen + "feature/plain.json", '{"type":"minecraft:simple_block"}')
        m = carry_fd.carry(self.fd, self.out, RULES)
        rules = {e["dest"]: e["rule"] for e in m["files"]}
        feature = json.loads((self.out / "data/farmersdelight/worldgen/feature/patch.json").read_text("utf-8"))
        self.assertEqual(feature["type"], "minecraft:overlay")
        self.assertEqual(feature["features"][0]["feature"]["type"], "minecraft:simple_block")
        placed = json.loads((self.out / "data/farmersdelight/worldgen/placed_feature/patch.json").read_text("utf-8"))
        self.assertEqual([p["type"] for p in placed["placement"]], ["minecraft:biome", "minecraft:offset"])
        self.assertEqual(rules["data/farmersdelight/worldgen/feature/patch.json"], "rewrite")
        self.assertEqual(rules["data/farmersdelight/worldgen/placed_feature/patch.json"], "rewrite")
        # a file no rule changes stays a byte copy
        self.assertEqual(rules["data/farmersdelight/worldgen/feature/plain.json"], "copy")
        self.assertEqual(
            (self.out / "data/farmersdelight/worldgen/feature/plain.json").read_text("utf-8"),
            '{"type":"minecraft:simple_block"}',
        )

    def test_addon_overlay(self):
        write(
            self.fd,
            "src/generated/resources/data/farmersdelight/tags/item/tools/knives.json",
            '{"values":["farmersdelight:flint_knife"]}',
        )
        overlay = Path(self.tmp.name) / "md.json"
        overlay.write_text(
            json.dumps(
                {
                    "addon": "moredelight",
                    "entries": [
                        {
                            "tag": "farmersdelight:tools/knives",
                            "kind": "item",
                            "ids": ["moredelight:stone_knife", "farmersdelight:flint_knife"],
                        }
                    ],
                }
            ),
            "utf-8",
        )
        pin = carry_fd.lib.sha256(overlay.read_bytes())
        rules = dict(RULES, overlay=[{"file": str(overlay), "sha256": pin}])
        m = carry_fd.carry(self.fd, self.out, rules)
        tag = json.loads((self.out / "data/farmersdelight/tags/item/tools/knives.json").read_text("utf-8"))
        self.assertEqual(
            tag["values"], ["farmersdelight:flint_knife", {"id": "moredelight:stone_knife", "required": False}]
        )
        self.assertIn("overlay", [e["rule"] for e in m["files"]])
        with self.assertRaises(carry_fd.CarryError):  # the pinned hash no longer matches
            carry_fd.carry(self.fd, self.out, dict(RULES, overlay=[{"file": str(overlay), "sha256": "0" * 64}]))
        (self.fd / "src/generated/resources/data/farmersdelight/tags/item/tools/knives.json").unlink()
        with self.assertRaises(carry_fd.CarryError):  # a tag the base does not carry
            carry_fd.carry(self.fd, self.out, rules)

    def test_base_tag_with_overlay(self):
        overlay = Path(self.tmp.name) / "md.json"
        overlay.write_text(
            json.dumps(
                {
                    "addon": "moredelight",
                    "entries": [
                        {"tag": "ported_farmersdelight:knife_items", "kind": "item", "ids": ["moredelight:stone_knife"]}
                    ],
                }
            ),
            "utf-8",
        )
        pin = carry_fd.lib.sha256(overlay.read_bytes())
        base_tag = [{"tag": "ported_farmersdelight:knife_items", "kind": "item", "values": ["farmersdelight:flint_knife"]}]
        rules = dict(RULES, base_tag=base_tag, overlay=[{"file": str(overlay), "sha256": pin}])
        m = carry_fd.carry(self.fd, self.out, rules)
        rel = "data/ported_farmersdelight/tags/item/knife_items.json"
        tag = json.loads((self.out / rel).read_text("utf-8"))
        self.assertEqual(
            tag["values"], ["farmersdelight:flint_knife", {"id": "moredelight:stone_knife", "required": False}]
        )
        self.assertEqual({e["dest"]: e["rule"] for e in m["files"]}[rel], "overlay")
        m = carry_fd.carry(self.fd, self.out, dict(RULES, base_tag=base_tag))
        self.assertEqual({e["dest"]: e["rule"] for e in m["files"]}[rel], "base_tag")


if __name__ == "__main__":
    unittest.main()
