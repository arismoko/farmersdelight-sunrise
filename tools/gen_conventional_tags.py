#!/usr/bin/env python3
"""Generate the Fabric-API convention item tags FD relies on (tools/conventional/item_tags.toml.in).

Writes data/c/tags/item/<name>.json with `"replace": false` so other mods' entries merge. Refuses to
write over a file the FD carry owns (tools/carry_manifest.json). `--check` verifies the files.
"""
import argparse
import json
import sys
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SRC = REPO / "tools" / "conventional" / "item_tags.toml.in"
OUT = REPO / "data" / "c" / "tags" / "item"


def render() -> dict[str, bytes]:
    with SRC.open("rb") as f:
        table = tomllib.load(f)
    carried = {e["dest"] for e in json.loads((REPO / "tools" / "carry_manifest.json").read_text())["files"]}
    out = {}
    for name, values in sorted(table.items()):
        rel = f"data/c/tags/item/{name}.json"
        if rel in carried:
            raise SystemExit(f"{rel} is carried from FD; refusing to generate it")
        out[rel] = (json.dumps({"replace": False, "values": values}, indent=2) + "\n").encode()
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    files = render()
    if args.check:
        bad = [r for r, b in files.items() if not (REPO / r).exists() or (REPO / r).read_bytes() != b]
        for r in bad:
            print(f"out of date: {r}", file=sys.stderr)
        if not bad:
            print(f"conventional tag check ok: {len(files)} files")
        return 1 if bad else 0
    for rel, body in files.items():
        p = REPO / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(body)
    print(f"wrote {len(files)} conventional tag file(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
