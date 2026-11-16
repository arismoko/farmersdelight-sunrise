#!/usr/bin/env python3
"""Unit tests for tools/carry_lib.py (BX-08): the parts the FD carry does not already exercise."""
import json
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import carry_lib as lib  # noqa: E402


class FenceTest(unittest.TestCase):
    def test_only_owned_and_shared_tag_folders(self):
        rels = [
            "assets/moredelight/lang/en_us.json",
            "data/moredelight/recipe/toast.json",
            "data/c/tags/item/bread_slices.json",
            "data/minecraft/tags/item/enchantable/sharp_weapon.json",
            "data/farmersdelight/tags/item/tools/knives.json",
            "data/farmersdelight/recipe/bacon_sandwich.json",
            "assets/minecraft/models/item/x.json",
            "data/minecraft/recipe/bread.json",
            "pack.mcmeta",
        ]
        self.assertEqual(
            lib.fence_violations(rels, {"moredelight"}),
            [
                "assets/minecraft/models/item/x.json",
                "data/farmersdelight/recipe/bacon_sandwich.json",
                "data/farmersdelight/tags/item/tools/knives.json",
                "data/minecraft/recipe/bread.json",
            ],
        )


class TagUniverseTest(unittest.TestCase):
    def test_definitions_from_tree_and_zip(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for rel in (
                "data/c/tags/item/foods/bread.json",
                "data/farmersdelight/tags/block/wild_crops.json",
                "data/c/tags/worldgen/biome/is_beach.json",
            ):
                (root / rel).parent.mkdir(parents=True, exist_ok=True)
                (root / rel).write_text("{}")
            self.assertEqual(
                lib.tags_in_tree(root),
                {
                    ("item", "c:foods/bread"),
                    ("block", "farmersdelight:wild_crops"),
                    ("worldgen/biome", "c:is_beach"),
                },
            )
            jar = root / "v.jar"
            with zipfile.ZipFile(jar, "w") as z:
                z.writestr("data/minecraft/tags/item/planks.json", "{}")
                z.writestr("data/minecraft/tags/item/", "")
                z.writestr("data/minecraft/recipe/bread.json", "{}")
            self.assertEqual(lib.tags_in_zip(jar), {("item", "minecraft:planks")})

    def test_references_and_report(self):
        recipe = json.loads(
            '{"ingredients":["#c:eggs",{"tag":"c:foods/bread"},"minecraft:bread",["#minecraft:planks"]],'
            '"tool":"#c:tools/knife","result":{"id":"moredelight:x"}}'
        )
        refs = lib.tag_references(recipe)
        self.assertEqual(refs, {"c:eggs", "c:foods/bread", "minecraft:planks", "c:tools/knife"})
        defined = {("item", "c:eggs"), ("item", "minecraft:planks"), ("item", "c:tools/knife")}
        self.assertEqual(lib.undefined_tags({"r.json": refs}, defined), {"r.json": ["c:foods/bread"]})
        self.assertEqual(lib.undefined_tags({"r.json": {"c:eggs"}}, defined), {})


class DumpTest(unittest.TestCase):
    def test_writer_format(self):
        self.assertEqual(lib.dump_json({"a": "é", "b": [1]}), '{\n  "a": "é",\n  "b": [\n    1\n  ]\n}\n'.encode())


if __name__ == "__main__":
    unittest.main()
