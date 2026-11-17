#!/usr/bin/env python3
"""Reusable worldgen rewrites for add-on carries (BX-09).

Upstream FD 1.21.1 (what Corn Delight builds on) generates wild crops with its own feature type
`farmersdelight:wild_crop` and filters biomes with `farmersdelight:biome_tag` and
`farmersdelight:add_features_by_filter`. FD Refabricated 26.3 restates the wild crop patch as data:
`farmersdelight:in_order` (== `minecraft:overlay`) of its floor, primary and secondary features, each
behind a trapezoid offset, with the tries moved into the placed feature as `minecraft:count`
(e.g. FD's `worldgen/feature/wild_carrots.json` + `placed_feature/patch_wild_carrots.json`). These
functions apply the same restatement, so an add-on's wild crop patch generates as FD 26.3's do:

- `wild_crop_to_vanilla(cfg)` → (`minecraft:overlay` feature, tries)
- `upgrade_feature(f)` → a 1.21.1 configured feature (`{type, config}`, `simple_state_provider`,
  `Name`/`Properties`) in 26.3 shape (`{type, …}`, `minecraft:simple`, `id`/`properties`)
- `placed_with_count(placed, tries)` / `drop_biome_tag(placed)` for the placed feature
- `biome_filter_to_neoforge(modifier)` → `neoforge:add_features` with the base's biome filter terms
- `biomes(allowed, denied, temperature)` → the filter term FD's own modifiers use (tools/gen_worldgen.py)

Pure functions on JSON values; no I/O.
"""
from __future__ import annotations

import copy
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import target  # noqa: E402


def trapezoid(spread: int) -> dict:
    """`random.nextInt(spread + 1) - random.nextInt(spread + 1)` as a 26.3 int provider."""
    return {"type": "minecraft:trapezoid", "max": spread, "min": -spread, "plateau": 0}


def spread_offset(xz: int, y: int) -> dict:
    return {"type": "minecraft:offset", "x": trapezoid(xz), "y": trapezoid(y), "z": trapezoid(xz)}


def upgrade_state(state: dict) -> dict:
    """1.21.1 block state `{Name, Properties}` → 26.3 `{id, properties}`."""
    out = {"id": state["Name"]}
    if state.get("Properties"):
        out["properties"] = dict(state["Properties"])
    return out


def upgrade_provider(p: dict) -> dict:
    if p.get("type") == "minecraft:simple_state_provider":
        return {"type": "minecraft:simple", "state": upgrade_state(p["state"])}
    raise ValueError(f"unsupported block state provider: {p.get('type')}")


def upgrade_feature(f: dict) -> dict:
    """A 1.21.1 configured feature → 26.3. Only `minecraft:simple_block` is needed (wild crop parts)."""
    if f.get("type") != "minecraft:simple_block":
        raise ValueError(f"unsupported feature type: {f.get('type')}")
    cfg = f.get("config", {})
    out = {"type": "minecraft:simple_block", "to_place": upgrade_provider(cfg["to_place"])}
    for k, v in cfg.items():
        if k != "to_place":
            out[k] = copy.deepcopy(v)
    return out


def _part(part: dict, lead: list[dict]) -> dict:
    """An inline placed feature (`{feature, placement}`) of the wild crop config, behind `lead`."""
    return {
        "feature": upgrade_feature(part["feature"]),
        "placement": lead + copy.deepcopy(part.get("placement", [])),
    }


def wild_crop_to_vanilla(feature: dict) -> tuple[dict, int]:
    """Upstream `farmersdelight:wild_crop` → (`minecraft:overlay` feature, tries).

    FD 26.3's order and offsets: floor first, one block up and spread ±xz/±y; primary spread ±(xz − 2)/±y;
    secondary spread ±xz/±y (WildCropFeature's `xzSpread + 1` bound and the `- 2` shorter primary spread).
    The tries become `minecraft:count` in the placed feature (`placed_with_count`)."""
    if feature.get("type") != "farmersdelight:wild_crop":
        raise ValueError(f"not a wild_crop feature: {feature.get('type')}")
    c = feature["config"]
    xz, y = c["xz_spread"], c["y_spread"]
    parts = []
    if "floor_feature" in c:
        parts.append(_part(c["floor_feature"], [{"type": "minecraft:offset", "x": 0, "y": 1, "z": 0}, spread_offset(xz, y)]))
    parts.append(_part(c["primary_feature"], [spread_offset(xz - 2, y)]))
    parts.append(_part(c["secondary_feature"], [spread_offset(xz, y)]))
    return {"type": "minecraft:overlay", "features": parts}, c["tries"]


def drop_biome_tag(placed: dict) -> dict:
    """Drop `farmersdelight:biome_tag` placement modifiers (FD M6-02: the biome modifier already limits
    the feature to overworld biomes, so `#minecraft:is_overworld` never rejects; it draws no random)."""
    out = copy.deepcopy(placed)
    out["placement"] = [p for p in out["placement"] if p.get("type") != "farmersdelight:biome_tag"]
    return out


def placed_with_count(placed: dict, tries: int) -> dict:
    """Insert `minecraft:count` right after `minecraft:in_square`, where FD 26.3's patches carry it."""
    out = copy.deepcopy(placed)
    mods = out["placement"]
    at = next((i + 1 for i, p in enumerate(mods) if p.get("type") == "minecraft:in_square"), 0)
    mods.insert(at, {"type": "minecraft:count", "count": tries})
    return out


def biomes(allowed, denied=None, temperature=None):
    """FD's biome selector (FDBiomeSelector) as one NeoForge biome filter: in `allowed`, not in `denied`,
    base temperature within [lo, hi] (`<engine>:climate`, tools/target.toml.in). `allowed`/`denied` are holder sets: a `#tag`, an
    id or a list of ids."""
    terms = [allowed]
    if denied:
        terms.append({"type": "neoforge:not", "value": denied})
    if temperature is not None:
        lo, hi = temperature
        terms.append({"type": target.key("climate"), "temperature": {"min": lo, "max": hi}})
    return terms[0] if len(terms) == 1 else {"type": "neoforge:and", "values": terms}


def biome_filter_to_neoforge(modifier: dict) -> dict:
    """Upstream `farmersdelight:add_features_by_filter` → `neoforge:add_features`. Temperatures default to
    −4…4 (no biome leaves it, so no climate term is written then)."""
    if modifier.get("type") != "farmersdelight:add_features_by_filter":
        raise ValueError(f"not an add_features_by_filter modifier: {modifier.get('type')}")
    lo = modifier.get("min_temperature", -4.0)
    hi = modifier.get("max_temperature", 4.0)
    temperature = None if (lo, hi) == (-4.0, 4.0) else (lo, hi)
    return {
        "type": "neoforge:add_features",
        "biomes": biomes(modifier["allowed_biomes"], modifier.get("denied_biomes"), temperature),
        "features": modifier["features"],
        "step": modifier.get("step", "vegetal_decoration"),
    }
