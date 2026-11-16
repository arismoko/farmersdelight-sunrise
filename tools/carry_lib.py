"""Reusable parts of the scripted carries (BX-08): the FD carry (`carry_fd.py`) and the add-on carries, which import
this file from a base checkout at their pinned commit (`--base <checkout>`).

- `dump_json`: the one JSON writer for rewritten files (2-space indent, UTF-8, trailing newline); untouched files
  stay byte copies.
- `rewrite_conditions`: `fabric:load_conditions` → `neoforge:conditions` by an exact-value rule table.
- `apply_rewrite` / `rewrite_json`: the `[[rewrite]]` rules (type rename, placement drop, tag-entry flattening).
- `collect`: files of several source roots, selected by prefix/exclude, failing on a path two roots share.
- manifest helpers (`write_manifest`, `read_manifest`, `remove_stale`) and `git_head`.
- `fence_violations`: the package namespace fence (Horizon refuses a package with a file in a namespace it does not
  own, except `data/minecraft/tags/**` and `data/c/tags/**`).
- tag universe: `tags_in_tree` / `tags_in_zip` list defined tags, `tag_references` lists the `#ns:path` references
  of a JSON file, `undefined_tags` reports the references no definer has.
"""
from __future__ import annotations

import fnmatch
import hashlib
import json
import subprocess
import tomllib
import zipfile
from pathlib import Path

FABRIC_KEY = "fabric:load_conditions"
NEOFORGE_KEY = "neoforge:conditions"
# Tag folders whose files define tags of that registry; `item` and `block` also hold nested paths.
FENCE_ALLOWED_PREFIXES = ("data/minecraft/tags/", "data/c/tags/")


class CarryError(Exception):
    pass


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def load_rules(path: Path) -> dict:
    with path.open("rb") as f:
        return tomllib.load(f)


def dump_json(obj) -> bytes:
    return (json.dumps(obj, indent=2, ensure_ascii=False) + "\n").encode("utf-8")


def git_head(checkout: Path) -> str | None:
    try:
        return subprocess.run(
            ["git", "-C", str(checkout), "rev-parse", "HEAD"], capture_output=True, text=True, check=True
        ).stdout.strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        return None


def selected(rel: str, include_prefixes, exclude) -> bool:
    if not any(rel.startswith(p) for p in include_prefixes):
        return False
    return not any(fnmatch.fnmatchcase(rel, pat) for pat in exclude)


def rewrite_conditions(node, condition_rules: list, rel: str):
    """Return (new_node, changed). Replaces every fabric:load_conditions key in place (order kept); a value no rule
    names exactly fails."""
    changed = False
    if isinstance(node, dict):
        out = {}
        for key, value in node.items():
            if key == FABRIC_KEY:
                for rule in condition_rules:
                    if value == rule["fabric"]:
                        out[NEOFORGE_KEY] = rule["neoforge"]
                        changed = True
                        break
                else:
                    raise CarryError(f"{rel}: unknown {FABRIC_KEY} value {json.dumps(value)}")
            else:
                new, c = rewrite_conditions(value, condition_rules, rel)
                out[key] = new
                changed |= c
        return out, changed
    if isinstance(node, list):
        items = [rewrite_conditions(v, condition_rules, rel) for v in node]
        return [i[0] for i in items], any(i[1] for i in items)
    return node, False


def apply_rewrite(node, rw: dict):
    """One [[rewrite]] rule over a JSON tree: rename a `type` value, drop entries of a `placement` list, or write a
    tag's `{"id": x, "required": false}` entries as plain "x" (flatten_tag_entries)."""
    if isinstance(node, dict):
        out = {}
        for key, value in node.items():
            if key == "values" and rw.get("flatten_tag_entries") and isinstance(value, list):
                value = [v["id"] if isinstance(v, dict) and set(v) <= {"id", "required"} else v for v in value]
            if key == "type" and "rename_type" in rw and value == rw["rename_type"]["from"]:
                value = rw["rename_type"]["to"]
            elif key == "placement" and "drop_placement" in rw and isinstance(value, list):
                value = [v for v in value if not (isinstance(v, dict) and v.get("type") == rw["drop_placement"])]
            out[key] = apply_rewrite(value, rw)
        return out
    if isinstance(node, list):
        return [apply_rewrite(v, rw) for v in node]
    return node


def rewrite_json(rel: str, data: bytes, rewrites: list) -> bytes | None:
    """The rewrite rules whose `glob` matches `rel`, applied in order; None when none matches or nothing changed."""
    matching = [rw for rw in rewrites if fnmatch.fnmatchcase(rel, rw["glob"])]
    if not matching:
        return None
    obj = json.loads(data.decode("utf-8"))
    new = obj
    for rw in matching:
        new = apply_rewrite(new, rw)
    if new == obj:
        return None
    return dump_json(new)


def collect(source: Path, roots, include_prefixes, exclude) -> dict[str, tuple[str, Path]]:
    """rel path -> (root, source path) over `roots` in order. Fails on a missing root or a path two roots share."""
    found: dict[str, tuple[str, Path]] = {}
    for root in roots:
        base = source / root
        if not base.is_dir():
            raise CarryError(f"missing source root {base}")
        for p in sorted(base.rglob("*")):
            if not p.is_file():
                continue
            rel = p.relative_to(base).as_posix()
            if not selected(rel, include_prefixes, exclude):
                continue
            if rel in found:
                raise CarryError(f"overlapping path in {found[rel][0]} and {root}: {rel}")
            found[rel] = (root, p)
    return found


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


def write_manifest(path: Path, manifest: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(manifest, indent=1) + "\n", "utf-8")


def read_manifest(path: Path) -> dict | None:
    return json.loads(path.read_text("utf-8")) if path.exists() else None


def fence_violations(rels, owned) -> list[str]:
    """Paths under `assets/<ns>/` or `data/<ns>/` with `ns` not in `owned`, except the shared tag folders."""
    bad = []
    for rel in rels:
        parts = rel.split("/")
        if len(parts) < 3 or parts[0] not in ("assets", "data"):
            continue
        if parts[1] in owned or rel.startswith(FENCE_ALLOWED_PREFIXES):
            continue
        bad.append(rel)
    return sorted(bad)


def _tag_of(rel: str) -> tuple[str, str] | None:
    """`data/<ns>/tags/<registry...>/<path>.json` → (registry, "ns:path"); registry is `item`, `block`,
    `worldgen/biome` …; None for anything else."""
    parts = rel.split("/")
    if len(parts) < 5 or parts[0] != "data" or parts[2] != "tags" or not rel.endswith(".json"):
        return None
    rest = parts[3:]
    # Registries are one folder (`item`) or `worldgen/<x>`.
    n = 2 if rest[0] == "worldgen" else 1
    if len(rest) <= n:
        return None
    registry = "/".join(rest[:n])
    path = "/".join(rest[n:])[: -len(".json")]
    return registry, f"{parts[1]}:{path}"


def tags_in_rels(rels) -> set[tuple[str, str]]:
    return {t for rel in rels if (t := _tag_of(rel)) is not None}


def tags_in_tree(root: Path) -> set[tuple[str, str]]:
    """Tags defined by the `data/` files under `root` (a mod tree)."""
    if not (root / "data").is_dir():
        return set()
    return tags_in_rels(p.relative_to(root).as_posix() for p in (root / "data").rglob("*.json"))


def tags_in_zip(jar: Path) -> set[tuple[str, str]]:
    """Tags defined by a jar's `data/` entries, read as a zip (never run)."""
    with zipfile.ZipFile(jar) as z:
        return tags_in_rels(z.namelist())


def tag_references(node) -> set[str]:
    """Every `#ns:path` string and `{"tag": "ns:path"}` value in a JSON tree, as "ns:path"."""
    found: set[str] = set()
    if isinstance(node, dict):
        for key, value in node.items():
            if key == "tag" and isinstance(value, str):
                found.add(value.removeprefix("#"))
            else:
                found |= tag_references(value)
    elif isinstance(node, list):
        for v in node:
            found |= tag_references(v)
    elif isinstance(node, str) and node.startswith("#") and ":" in node:
        found.add(node[1:])
    return found


def undefined_tags(references: dict[str, set[str]], defined: set[tuple[str, str]]) -> dict[str, list[str]]:
    """file → its referenced tags that no registry defines. References carry no registry, so a tag counts as defined
    when any registry has it (a reference report, not a type check)."""
    names = {name for _, name in defined}
    out = {}
    for rel, refs in sorted(references.items()):
        missing = sorted(r for r in refs if r not in names)
        if missing:
            out[rel] = missing
    return out
