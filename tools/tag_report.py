#!/usr/bin/env python3
"""Tag-universe report (BX-01, AD6): every `#ns:path` a mod tree's data references, and whether vanilla 26.3, the
base or the tree itself defines it.

Usage:
  tag_report.py --vanilla <minecraft-26.3-client.jar> [--tree <mod tree> ...] [--only c]

The base (this repository) is always part of the universe; each `--tree` adds its own definitions and references.
Exit 1 when a reference is undefined. The jar is read as a zip, never run.
"""
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import carry_lib as lib  # noqa: E402

REPO = Path(__file__).resolve().parent.parent


def references(tree: Path) -> dict[str, set[str]]:
    out = {}
    for p in sorted((tree / "data").rglob("*.json")):
        try:
            refs = lib.tag_references(json.loads(p.read_text("utf-8")))
        except (json.JSONDecodeError, UnicodeDecodeError):
            continue
        if refs:
            out[f"{tree.name}/{p.relative_to(tree).as_posix()}"] = refs
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--vanilla", required=True, type=Path)
    ap.add_argument("--tree", action="append", type=Path, default=[])
    ap.add_argument("--only", help="report only references in this namespace (e.g. c)")
    args = ap.parse_args()
    trees = [REPO, *args.tree]
    defined = lib.tags_in_zip(args.vanilla)
    refs: dict[str, set[str]] = {}
    for t in trees:
        defined |= lib.tags_in_tree(t)
        refs.update(references(t))
    if args.only:
        refs = {k: {r for r in v if r.startswith(args.only + ":")} for k, v in refs.items()}
    missing = lib.undefined_tags(refs, defined)
    used = sorted({r for v in refs.values() for r in v})
    print(f"{len(used)} referenced tags, {len(missing)} file(s) with undefined ones")
    for rel, tags in missing.items():
        print(f"  {rel}: {', '.join(tags)}")
    return 1 if missing else 0


if __name__ == "__main__":
    sys.exit(main())
