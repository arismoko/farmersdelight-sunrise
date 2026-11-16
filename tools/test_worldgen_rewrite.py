#!/usr/bin/env python3
"""Tests for tools/worldgen_rewrite.py (BX-09): the upstream FD 1.21.1 wild_crop shape, rewritten,
equals FD 26.3's own wild carrot patch as the base carries it."""
import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import worldgen_rewrite as w  # noqa: E402

DATA = Path(__file__).resolve().parent.parent / "data" / "farmersdelight" / "worldgen"


def simple_block(block: str) -> dict:
    return {"type": "minecraft:simple_block",
            "config": {"to_place": {"type": "minecraft:simple_state_provider", "state": {"Name": block}}}}


def on(tag: str) -> list[dict]:
    return [{"type": "minecraft:block_predicate_filter", "predicate": {"type": "minecraft:all_of", "predicates": [
        {"type": "minecraft:matching_blocks", "blocks": "minecraft:air"},
        {"type": "minecraft:matching_block_tag", "tag": tag, "offset": [0, -1, 0]}]}}]


# Upstream FD 1.21.1's wild carrots (`wildCropFeature(WILD_CARROTS, SHORT_GRASS, COARSE_DIRT, …)`, tries 64,
# xz 6, y 3) in its configured_feature JSON shape, with 26.3's substrate tag.
UPSTREAM_CARROTS = {
    "type": "farmersdelight:wild_crop",
    "config": {
        "primary_feature": {"feature": simple_block("farmersdelight:wild_carrots"),
                            "placement": on("minecraft:substrate_overworld")},
        "secondary_feature": {"feature": simple_block("minecraft:short_grass"),
                              "placement": on("minecraft:substrate_overworld")},
        "floor_feature": {"feature": simple_block("minecraft:coarse_dirt"), "placement": [
            {"type": "minecraft:block_predicate_filter", "predicate": {"type": "minecraft:all_of", "predicates": [
                {"type": "minecraft:replaceable", "offset": [0, 1, 0]},
                {"type": "minecraft:matching_block_tag", "tag": "minecraft:substrate_overworld"}]}}]},
        "tries": 64, "xz_spread": 6, "y_spread": 3,
    },
}


class WildCrop(unittest.TestCase):
    def test_reproduces_fd_wild_carrots(self):
        feature, tries = w.wild_crop_to_vanilla(UPSTREAM_CARROTS)
        carried = json.loads((DATA / "feature" / "wild_carrots.json").read_text())
        self.assertEqual(feature, carried)
        self.assertEqual(tries, 64)

    def test_placed_feature_matches_fd(self):
        upstream = {"feature": "farmersdelight:wild_carrots", "placement": [
            {"type": "minecraft:rarity_filter", "chance": 120}, {"type": "minecraft:in_square"},
            {"type": "minecraft:heightmap", "heightmap": "MOTION_BLOCKING"}, {"type": "minecraft:biome"},
            {"type": "farmersdelight:biome_tag", "tag": "minecraft:is_overworld"}]}
        placed = w.placed_with_count(w.drop_biome_tag(upstream), 64)
        carried = json.loads((DATA / "placed_feature" / "patch_wild_carrots.json").read_text())
        self.assertEqual(placed, carried)

    def test_no_floor_and_properties(self):
        cfg = json.loads(json.dumps(UPSTREAM_CARROTS))
        del cfg["config"]["floor_feature"]
        cfg["config"]["primary_feature"]["feature"]["config"]["to_place"]["state"]["Properties"] = {"age": "0"}
        feature, _ = w.wild_crop_to_vanilla(cfg)
        self.assertEqual(len(feature["features"]), 2)
        self.assertEqual(feature["features"][0]["feature"]["to_place"]["state"],
                         {"id": "farmersdelight:wild_carrots", "properties": {"age": "0"}})
        self.assertEqual(feature["features"][0]["placement"][0]["x"]["max"], 4)

    def test_rejects_other_types(self):
        with self.assertRaises(ValueError):
            w.wild_crop_to_vanilla({"type": "minecraft:random_patch", "config": {}})
        with self.assertRaises(ValueError):
            w.upgrade_feature({"type": "minecraft:tree", "config": {}})


class Biomes(unittest.TestCase):
    def test_filter_modifier(self):
        m = w.biome_filter_to_neoforge({
            "type": "farmersdelight:add_features_by_filter", "allowed_biomes": "#minecraft:is_overworld",
            "denied_biomes": ["minecraft:lush_caves", "minecraft:mushroom_fields"],
            "min_temperature": 0.4, "max_temperature": 0.9, "features": "testaddon:patch_wild_x",
            "step": "vegetal_decoration"})
        self.assertEqual(m, {"type": "neoforge:add_features", "biomes": {"type": "neoforge:and", "values": [
            "#minecraft:is_overworld",
            {"type": "neoforge:not", "value": ["minecraft:lush_caves", "minecraft:mushroom_fields"]},
            {"type": "horizon:climate", "temperature": {"min": 0.4, "max": 0.9}}]},
            "features": "testaddon:patch_wild_x", "step": "vegetal_decoration"})

    def test_default_temperature_writes_no_climate(self):
        m = w.biome_filter_to_neoforge({"type": "farmersdelight:add_features_by_filter",
                                        "allowed_biomes": "#c:is_plains", "features": "x:y"})
        self.assertEqual(m["biomes"], "#c:is_plains")


if __name__ == "__main__":
    unittest.main()
