#!/usr/bin/env python3
"""Deterministically carry core Farmer's Delight data/assets into this repo (PORT_PLAN D6, M0-05).

Usage:
  carry_fd.py --fd <FarmersDelightRefabricated checkout>            # write into the repo
  carry_fd.py --fd <checkout> --check                               # verify the repo matches (CI)
  carry_fd.py --fd <checkout> --out <dir>                           # write into another tree

Files are copied byte for byte; content changes are the fabric:load_conditions rewrite, the scripted
[[rewrite]] rules and the [[override]] files listed in tools/carry_rules.toml.in. tools/carry_manifest.json records every carried file.
"""
from __future__ import annotations

import argparse
import json
import sys
import tempfile
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import carry_lib as lib  # noqa: E402

REPO = Path(__file__).resolve().parent.parent
RULES = REPO / "tools" / "carry_rules.toml.in"
MANIFEST_REL = "tools/carry_manifest.json"


def load_rules(path: Path) -> dict:
    return lib.load_rules(path)


def collect(fd: Path, rules: dict) -> dict[str, tuple[str, Path]]:
    src = rules["source"]
    return lib.collect(fd, src["roots"], src["include_prefixes"], src["exclude"])


def carry(fd: Path, out: Path, rules: dict) -> dict:
    files = collect(fd, rules)
    entries = []
    for rel in sorted(files):
        root, src = files[rel]
        data = src.read_bytes()
        rule = "copy"
        result = data
        if rel.endswith(".json") and lib.FABRIC_KEY.encode() in data:
            obj = json.loads(data.decode("utf-8"))
            new, changed = lib.rewrite_conditions(obj, rules.get("load_condition", []), rel)
            if changed:
                result = lib.dump_json(new)
                rule = "load_conditions"
        if rel.endswith(".json"):
            rewritten = lib.rewrite_json(rel, result, rules.get("rewrite", []))
            if rewritten is not None:
                result = rewritten
                rule = "rewrite"
        overrides = {o["dest"] for o in rules.get("override", [])}
        if rel in overrides:
            result = (REPO / "tools" / "carry_overrides" / rel).read_bytes()
            rule = "override"
        dest = out / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        if not dest.exists() or dest.read_bytes() != result:
            dest.write_bytes(result)
        entries.append(
            {
                "source": f"{root}/{rel}",
                "dest": rel,
                "sha256_source": lib.sha256(data),
                "sha256_dest": lib.sha256(result),
                "rule": rule,
            }
        )
    manifest = {
        "fd_repo": rules["source"]["repo"],
        "fd_sha": rules["source"]["sha"],
        "rules_sha256": lib.sha256(RULES.read_bytes()),
        "files": entries,
    }
    return manifest


remove_stale = lib.remove_stale
CarryError = lib.CarryError


def counts(manifest: dict) -> list[tuple[str, int]]:
    c: Counter[str] = Counter()
    for e in manifest["files"]:
        d = e["dest"]
        parts = d.split("/")
        if d.startswith("data/farmersdelight/recipe/"):
            c["recipes"] += 1
        if len(parts) > 2 and parts[0] == "data" and parts[2] == "tags":
            c["tag files"] += 1
            c[f"tag files ({parts[1]})"] += 1
        if d.startswith("data/farmersdelight/loot_table/"):
            c["loot tables"] += 1
        if d.startswith("data/farmersdelight/advancement/"):
            c["advancements"] += 1
        if d.startswith("assets/farmersdelight/blockstates/"):
            c["blockstates"] += 1
        if d.startswith("assets/farmersdelight/models/block/"):
            c["block models"] += 1
        if d.startswith("assets/farmersdelight/models/item/"):
            c["item models"] += 1
        if d.startswith("assets/farmersdelight/items/"):
            c["items/ definitions"] += 1
        if d.endswith(".png"):
            c["png"] += 1
        if d.endswith(".mcmeta"):
            c["mcmeta"] += 1
        if d.startswith("assets/farmersdelight/lang/"):
            c["lang"] += 1
        if d.startswith("assets/farmersdelight/sounds/"):
            c["ogg"] += 1
        if d.startswith("data/farmersdelight/worldgen/"):
            c["worldgen"] += 1
        if d.startswith("data/farmersdelight/structure/"):
            c["structures"] += 1
        if d.startswith("data/farmersdelight/villager_trade/"):
            c["villager trades"] += 1
        if e["rule"] == "load_conditions":
            c["rewritten"] += 1
        if e["rule"] == "override":
            c["overridden"] += 1
        if e["rule"] == "rewrite":
            c["rewritten (worldgen)"] += 1
    c["total files"] = len(manifest["files"])
    return sorted(c.items())


def sound_events(out: Path) -> int:
    p = out / "assets/farmersdelight/sounds.json"
    return len(json.loads(p.read_text("utf-8"))) if p.exists() else 0


def write_manifest(out: Path, manifest: dict) -> None:
    lib.write_manifest(out / MANIFEST_REL, manifest)


def read_manifest(out: Path) -> dict | None:
    return lib.read_manifest(out / MANIFEST_REL)


def check(fd: Path, rules: dict) -> int:
    with tempfile.TemporaryDirectory() as tmp:
        tmpdir = Path(tmp)
        manifest = carry(fd, tmpdir, rules)
        write_manifest(tmpdir, manifest)
        problems = []
        if (REPO / MANIFEST_REL).read_bytes() != (tmpdir / MANIFEST_REL).read_bytes():
            problems.append(f"{MANIFEST_REL} differs")
        for e in manifest["files"]:
            ours = REPO / e["dest"]
            if not ours.exists():
                problems.append(f"missing {e['dest']}")
            elif lib.sha256(ours.read_bytes()) != e["sha256_dest"]:
                problems.append(f"drift {e['dest']}")
    for p in problems[:50]:
        print(p, file=sys.stderr)
    if problems:
        print(f"carry check FAILED: {len(problems)} problem(s)", file=sys.stderr)
        return 1
    print(f"carry check ok: {len(manifest['files'])} files")
    return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--fd", required=True, type=Path, help="FarmersDelightRefabricated checkout")
    ap.add_argument("--out", type=Path, default=REPO)
    ap.add_argument("--check", action="store_true")
    ap.add_argument("--rules", type=Path, default=RULES)
    args = ap.parse_args(argv)
    rules = load_rules(args.rules)
    head = lib.git_head(args.fd)
    if head != rules["source"]["sha"]:
        print(f"FD checkout is at {head}, rules pin {rules['source']['sha']}", file=sys.stderr)
        return 2
    try:
        if args.check:
            return check(args.fd, rules)
        old = read_manifest(args.out)
        manifest = carry(args.fd, args.out, rules)
        stale = remove_stale(args.out, old, manifest)
        write_manifest(args.out, manifest)
    except CarryError as e:
        print(f"carry FAILED: {e}", file=sys.stderr)
        return 1
    for name, n in counts(manifest):
        print(f"{name:28} {n}")
    print(f"{'sound events':28} {sound_events(args.out)}")
    if stale:
        print(f"removed {len(stale)} stale file(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
