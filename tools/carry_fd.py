#!/usr/bin/env python3
"""Deterministically carry core Farmer's Delight data/assets into this repo (PORT_PLAN D6, M0-05).

Usage:
  carry_fd.py --fd <FarmersDelightRefabricated checkout>            # write into the repo
  carry_fd.py --fd <checkout> --check                               # verify the repo matches (CI)
  carry_fd.py --fd <checkout> --out <dir>                           # write into another tree

Files are copied byte for byte; the only content change is the fabric:load_conditions rewrite
from tools/carry_rules.toml. tools/carry_manifest.json records every carried file.
"""
from __future__ import annotations

import argparse
import fnmatch
import hashlib
import json
import shutil
import subprocess
import sys
import tempfile
import tomllib
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
RULES = REPO / "tools" / "carry_rules.toml"
MANIFEST_REL = "tools/carry_manifest.json"
FABRIC_KEY = "fabric:load_conditions"
NEOFORGE_KEY = "neoforge:conditions"


class CarryError(Exception):
    pass


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def load_rules(path: Path) -> dict:
    with path.open("rb") as f:
        return tomllib.load(f)


def selected(rel: str, rules: dict) -> bool:
    if not any(rel.startswith(p) for p in rules["source"]["include_prefixes"]):
        return False
    return not any(fnmatch.fnmatchcase(rel, pat) for pat in rules["source"]["exclude"])


def rewrite_conditions(node, rules: dict, rel: str):
    """Return (new_node, changed). Replaces every fabric:load_conditions key in place (order kept)."""
    changed = False
    if isinstance(node, dict):
        out = {}
        for key, value in node.items():
            if key == FABRIC_KEY:
                for rule in rules.get("load_condition", []):
                    if value == rule["fabric"]:
                        out[NEOFORGE_KEY] = rule["neoforge"]
                        changed = True
                        break
                else:
                    raise CarryError(f"{rel}: unknown {FABRIC_KEY} value {json.dumps(value)}")
            else:
                new, c = rewrite_conditions(value, rules, rel)
                out[key] = new
                changed |= c
        return out, changed
    if isinstance(node, list):
        items = [rewrite_conditions(v, rules, rel) for v in node]
        return [i[0] for i in items], any(i[1] for i in items)
    return node, False


def collect(fd: Path, rules: dict) -> dict[str, tuple[str, Path]]:
    """rel path -> (root, source path). Fails on overlap between roots."""
    found: dict[str, tuple[str, Path]] = {}
    for root in rules["source"]["roots"]:
        base = fd / root
        if not base.is_dir():
            raise CarryError(f"missing source root {base}")
        for p in sorted(base.rglob("*")):
            if not p.is_file():
                continue
            rel = p.relative_to(base).as_posix()
            if not selected(rel, rules):
                continue
            if rel in found:
                raise CarryError(f"overlapping path in {found[rel][0]} and {root}: {rel}")
            found[rel] = (root, p)
    return found


def carry(fd: Path, out: Path, rules: dict) -> dict:
    files = collect(fd, rules)
    entries = []
    for rel in sorted(files):
        root, src = files[rel]
        data = src.read_bytes()
        rule = "copy"
        result = data
        if rel.endswith(".json") and FABRIC_KEY.encode() in data:
            obj = json.loads(data.decode("utf-8"))
            new, changed = rewrite_conditions(obj, rules, rel)
            if changed:
                result = (json.dumps(new, indent=2, ensure_ascii=False) + "\n").encode("utf-8")
                rule = "load_conditions"
        dest = out / rel
        dest.parent.mkdir(parents=True, exist_ok=True)
        if not dest.exists() or dest.read_bytes() != result:
            dest.write_bytes(result)
        entries.append(
            {
                "source": f"{root}/{rel}",
                "dest": rel,
                "sha256_source": sha256(data),
                "sha256_dest": sha256(result),
                "rule": rule,
            }
        )
    manifest = {
        "fd_repo": rules["source"]["repo"],
        "fd_sha": rules["source"]["sha"],
        "rules_sha256": sha256(RULES.read_bytes()),
        "files": entries,
    }
    return manifest


def remove_stale(out: Path, old: dict | None, new: dict) -> list[str]:
    if not old:
        return []
    keep = {e["dest"] for e in new["files"]}
    stale = sorted(e["dest"] for e in old["files"] if e["dest"] not in keep)
    for rel in stale:
        p = out / rel
        if p.exists():
            p.unlink()
    return stale


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
        if e["rule"] != "copy":
            c["rewritten"] += 1
    c["total files"] = len(manifest["files"])
    return sorted(c.items())


def sound_events(out: Path) -> int:
    p = out / "assets/farmersdelight/sounds.json"
    return len(json.loads(p.read_text("utf-8"))) if p.exists() else 0


def write_manifest(out: Path, manifest: dict) -> None:
    p = out / MANIFEST_REL
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps(manifest, indent=1) + "\n", "utf-8")


def read_manifest(out: Path) -> dict | None:
    p = out / MANIFEST_REL
    return json.loads(p.read_text("utf-8")) if p.exists() else None


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
            elif sha256(ours.read_bytes()) != e["sha256_dest"]:
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
    try:
        head = subprocess.run(
            ["git", "-C", str(args.fd), "rev-parse", "HEAD"], capture_output=True, text=True, check=True
        ).stdout.strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        head = None
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
