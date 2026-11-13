#!/usr/bin/env python3
"""Unit tests for tools/carry_fd.py on a tiny fixture tree."""
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import carry_fd  # noqa: E402

RULES = carry_fd.load_rules(carry_fd.RULES)


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


if __name__ == "__main__":
    unittest.main()
