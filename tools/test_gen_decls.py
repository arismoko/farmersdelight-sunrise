#!/usr/bin/env python3
"""Unit tests for tools/gen_decls.py."""
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import gen_decls  # noqa: E402


class GenDeclsTest(unittest.TestCase):
    def run_on(self, files: dict[str, str]) -> dict[str, bytes]:
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            for name, text in files.items():
                (d / name).write_text(text, "utf-8")
            return gen_decls.render(d)

    def test_item_and_block(self):
        out = self.run_on({
            "items.toml": '[knife]\nsource = "J/X.java:A"\ntemplate = "minecraft:stick"\n'
                          'components = { "minecraft:max_damage" = 131 }\n',
            "blocks.toml": '[board]\nsource = "J/B.java"\ntemplate = "minecraft:oak_planks"\n'
                           'properties = { hardness = 2.0, occludes = false }\n',
        })
        self.assertEqual(sorted(out), ["block/board.json", "item/knife.json"])
        item = json.loads(out["item/knife.json"])
        self.assertEqual(item, {"template": "minecraft:stick", "components": {"minecraft:max_damage": 131}})
        self.assertNotIn("source", json.loads(out["block/board.json"]))

    def test_deterministic(self):
        files = {"items.toml": '[a]\nsource = "s"\ntemplate = "minecraft:stick"\nmax_stack_size = 16\n'}
        self.assertEqual(self.run_on(files), self.run_on(files))

    def test_rejects_unknown_key(self):
        with self.assertRaises(gen_decls.DeclError):
            self.run_on({"items.toml": '[a]\nsource = "s"\ntemplate = "minecraft:stick"\nlight = 3\n'})

    def test_rejects_unknown_block_property(self):
        with self.assertRaises(gen_decls.DeclError):
            self.run_on({"blocks.toml": '[a]\nsource = "s"\ntemplate = "minecraft:stone"\n'
                                        'properties = { glow = 1 }\n'})

    def test_requires_source(self):
        with self.assertRaises(gen_decls.DeclError):
            self.run_on({"items.toml": '[a]\ntemplate = "minecraft:stick"\n'})


if __name__ == "__main__":
    unittest.main()
