#!/usr/bin/env python3
"""Write parity fixtures from the pinned Farmer's Delight checkout (PORT_PLAN M0-09, VALIDATION_PLAN §2.3).

Data-derived (JSON parse of FD data): cutting_recipes, cooking_recipes, heat_tags.
Java-derived (FD is MIT; parsed from source text, each row cites its symbol): foods, config_defaults.
Transcribed (tools/fixture_src/*.toml.in, each row cites its symbol): knives.
loot_rules is transcribed in M8-05 (not produced yet).

Usage: extract_fixtures.py --fd <checkout> [--check]
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
OUT = REPO / "tests" / "fixtures"
SRC = REPO / "tools" / "fixture_src"
FD_SHA = "1b85d66f5e2aba88592d7f4fb203d582f4d3cbe3"
GEN = "src/generated/resources/data/farmersdelight"
JAVA = "src/main/java/vectorwing/farmersdelight"


def rel(fd: Path, p: Path) -> str:
    return p.relative_to(fd).as_posix()


def recipes(fd: Path, kind: str) -> list[dict]:
    rows = []
    for p in sorted((fd / GEN / "recipe").rglob("*.json")):
        obj = json.loads(p.read_text("utf-8"))
        if obj.get("type") != f"farmersdelight:{kind}":
            continue
        rid = "farmersdelight:" + p.relative_to(fd / GEN / "recipe").with_suffix("").as_posix()
        rows.append({"id": rid, "source": rel(fd, p), "json": obj})
    return rows


def heat_tags(fd: Path) -> dict:
    out = {}
    for name in ("heat_sources", "heat_conductors", "tray_heat_sources"):
        p = fd / GEN / "tags/block" / f"{name}.json"
        out[f"farmersdelight:{name}"] = {"source": rel(fd, p), "values": json.loads(p.read_text("utf-8"))["values"]}
    return out


FOOD_RE = re.compile(
    r"public static final FoodProperties (\w+)\s*=\s*\(new FoodProperties\.Builder\(\)\)(.*?)\.build\(\);", re.S
)


def foods(fd: Path) -> list[dict]:
    p = fd / JAVA / "common/FoodValues.java"
    text = p.read_text("utf-8")
    rows = []
    for m in FOOD_RE.finditer(text):
        name, body = m.group(1), m.group(2)
        nut = re.search(r"\.nutrition\((\d+)\)", body)
        sat = re.search(r"\.saturationModifier\(([\d.]+)f?\)", body)
        n = int(nut.group(1)) if nut else 0
        s = float(sat.group(1)) if sat else 0.0
        rows.append(
            {
                "constant": name,
                "source": f"{rel(fd, p)}:{name}",
                "nutrition": n,
                "saturation_modifier": s,
                "saturation": round(n * s * 2.0, 6),
                "always_edible": ".alwaysEdible()" in body,
            }
        )
    # direct record form: new FoodProperties(nutrition, saturation (absolute), canAlwaysEat)
    for m in DIRECT_FOOD_RE.finditer(text):
        name, n, sat, always = m.group(1), int(m.group(2)), float(m.group(3)), m.group(4) == "true"
        rows.append(
            {
                "constant": name,
                "source": f"{rel(fd, p)}:{name}",
                "nutrition": n,
                "saturation_modifier": round(sat / (2.0 * n), 6) if n else 0.0,
                "saturation": sat,
                "always_edible": always,
            }
        )
    rows.sort(key=lambda r: text.index(f"FoodProperties {r['constant']} "))
    return rows


DIRECT_FOOD_RE = re.compile(
    r"public static final FoodProperties (\w+)\s*=\s*new FoodProperties\((\d+),\s*([\d.]+)f?,\s*(true|false)\);"
)
DEFINE_RE = re.compile(r'\.define(?:InRange|List)?\("(\w+)",\s*(.*?)\);', re.S)
TOKEN_RE = re.compile(r'(\w+_BUILDER)\.(?:comment\([^;]*?\)\s*\.)?push\("?([\w.]*)"?\)|(\w+_BUILDER)\.pop\(\)|' + DEFINE_RE.pattern, re.S)


def java_value(raw: str):
    raw = raw.strip()
    lst = re.match(r"ImmutableList\.of\((.*)\)$", raw, re.S)
    if lst:
        return [s.strip().strip('"') for s in lst.group(1).split(",") if s.strip()]
    first = raw.split(",")[0].strip()
    if first in ("true", "false"):
        return first == "true"
    if first.startswith('"'):
        return first.strip('"')
    try:
        return int(first)
    except ValueError:
        return float(first.rstrip("fFdD"))


def config_defaults(fd: Path) -> dict:
    p = fd / JAVA / "common/Configuration.java"
    text = p.read_text("utf-8")
    consts = dict(re.findall(r'public static final String (CATEGORY_\w+)\s*=\s*"([^"]+)"', text))
    text = re.sub(r"\b(CATEGORY_\w+)\b", lambda m: f'"{consts.get(m.group(1), m.group(1))}"', text)
    out: dict[str, list] = {"common": [], "client": []}
    stacks: dict[str, list[str]] = {}
    for m in TOKEN_RE.finditer(text):
        if m.group(1):
            stacks.setdefault(m.group(1), []).append(m.group(2))
        elif m.group(3):
            stacks.setdefault(m.group(3), [])
            if stacks[m.group(3)]:
                stacks[m.group(3)].pop()
        else:
            key, raw = m.group(4), m.group(5)
            start = text.rfind("_BUILDER", 0, m.start())
            builder = text[text.rfind("\n", 0, start) :start].split()[-1].split("=")[-1].strip() + "_BUILDER"
            side = "client" if builder.startswith("CLIENT") else "common"
            out[side].append(
                {
                    "key": ".".join(stacks.get(builder, []) + [key]),
                    "default": java_value(raw),
                    "source": f"{rel(fd, p)}:{key}",
                }
            )
    return out


def knives() -> dict:
    with (SRC / "knives.toml.in").open("rb") as f:
        return tomllib.load(f)


def render(fd: Path) -> dict[str, bytes]:
    data = {
        "cutting_recipes": recipes(fd, "cutting"),
        "cooking_recipes": recipes(fd, "cooking"),
        "heat_tags": heat_tags(fd),
        "foods": foods(fd),
        "config_defaults": config_defaults(fd),
        "knives": knives(),
    }
    out = {}
    for name, body in data.items():
        doc = {"fd_sha": FD_SHA, "generated_by": "tools/extract_fixtures.py", name: body}
        out[f"{name}.json"] = (json.dumps(doc, indent=1, ensure_ascii=False, sort_keys=False) + "\n").encode()
    return out


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--fd", required=True, type=Path)
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args(argv)
    head = subprocess.run(["git", "-C", str(args.fd), "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
    if head != FD_SHA:
        print(f"FD checkout at {head!r}, expected {FD_SHA}", file=sys.stderr)
        return 2
    files = render(args.fd)
    if args.check:
        bad = [n for n, b in files.items() if not (OUT / n).exists() or (OUT / n).read_bytes() != b]
        for n in bad:
            print(f"out of date: tests/fixtures/{n}", file=sys.stderr)
        if not bad:
            print(f"fixture check ok: {len(files)} files")
        return 1 if bad else 0
    OUT.mkdir(parents=True, exist_ok=True)
    for n, b in files.items():
        (OUT / n).write_bytes(b)
    for n in files:
        doc = json.loads(files[n])
        body = doc[n[:-5]]
        size = len(body) if not isinstance(body, dict) or n != "config_defaults.json" else {k: len(v) for k, v in body.items()}
        print(f"{n:24} {size}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
