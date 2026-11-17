#!/usr/bin/env python3
"""M8-01: the six knife declarations follow FD's knifeItem(material) with vanilla 26.3 tool materials.

Material values are from the vanilla 26.3 datagen components report (<m>_sword / <m>_pickaxe):
sword damage = 3 + bonus; pickaxe tool speed = material speed. Flint is FD's own ModToolMaterials.FLINT.
"""
import json, pathlib, sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
# id: (durability, speed, damage bonus, enchantability, incorrect tag, repair tag, fire resistant)
MATERIALS = {
    "flint": (131, 4.0, 1.0, 5, "wooden", "#farmersdelight:flint_tool_materials", False),
    "copper": (190, 5.0, 1.0, 13, "copper", "#minecraft:copper_tool_materials", False),
    "iron": (250, 6.0, 2.0, 14, "iron", "#minecraft:iron_tool_materials", False),
    "diamond": (1561, 8.0, 3.0, 10, "diamond", "#minecraft:diamond_tool_materials", False),
    "netherite": (2031, 9.0, 4.0, 15, "netherite", "#minecraft:netherite_tool_materials", True),
    "golden": (32, 12.0, 0.0, 22, "gold", "#minecraft:gold_tool_materials", False),
}

def main():
    bad = []
    for name, (dur, speed, bonus, ench, inc, rep, fire) in MATERIALS.items():
        d = json.loads((ROOT / f"data/farmersdelight/sunrise/item/{name}_knife.json").read_text())
        c = d["components"]
        mods = {m["type"]: m["amount"] for m in c["minecraft:attribute_modifiers"]}
        rules = c["minecraft:tool"]["rules"]
        got = (c["minecraft:max_damage"], rules[1]["speed"], mods["minecraft:attack_damage"] - 0.5,
               c["minecraft:enchantable"]["value"], rules[0]["blocks"], c["minecraft:repairable"]["items"],
               c.get("minecraft:damage_resistant") == {"types": "#minecraft:is_fire"})
        want = (dur, speed, bonus, ench, f"#minecraft:incorrect_for_{inc}_tool", rep, fire)
        if got != want or mods["minecraft:attack_speed"] != -2.0 or d["max_stack_size"] != 1:
            bad.append(f"{name}: {got} != {want}")
    if bad:
        print("\n".join(bad)); sys.exit(1)
    print("knife decls ok: 6")

if __name__ == "__main__":
    main()
