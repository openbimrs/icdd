#!/usr/bin/env python3
"""Fail closed unless `icdd` is a semantic pure alias of `openbim-icdd`."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def fail(message: str) -> "NoReturn":
    print(f"alias purity: {message}", file=sys.stderr)
    raise SystemExit(1)


def package(packages: list[dict], name: str) -> dict:
    matches = [candidate for candidate in packages if candidate["name"] == name]
    if len(matches) != 1:
        fail(f"expected exactly one {name!r} package, found {len(matches)}")
    return matches[0]


def normalized(path: str | Path) -> Path:
    return Path(path).resolve()


metadata = json.loads(
    subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
)
packages = metadata["packages"]
canonical = package(packages, "openbim-icdd")
alias = package(packages, "icdd")

canonical_version = canonical["version"]
alias_version = alias["version"]
if alias_version != canonical_version:
    fail(
        f"package versions differ: icdd={alias_version}, "
        f"openbim-icdd={canonical_version}"
    )

expected_alias_manifest = normalized(ROOT / "icdd/Cargo.toml")
if normalized(alias["manifest_path"]) != expected_alias_manifest:
    fail(f"icdd manifest moved outside {expected_alias_manifest}")

if alias.get("features"):
    fail("icdd must not define features")
if alias.get("links") is not None:
    fail("icdd must not define a native links contract")

if len(alias["targets"]) != 1:
    fail("icdd must contain exactly one Cargo target")
target = alias["targets"][0]
if target["kind"] != ["lib"] or target["crate_types"] != ["lib"]:
    fail("icdd's only target must be a normal library")
if target["name"] != "icdd":
    fail(f"icdd library target has unexpected name {target['name']!r}")

source_path = normalized(target["src_path"])
expected_source_path = normalized(ROOT / "icdd/src/lib.rs")
if source_path != expected_source_path:
    fail(f"icdd library target must be {expected_source_path}, got {source_path}")

meaningful_lines = [
    line.strip()
    for line in source_path.read_text(encoding="utf-8").splitlines()
    if line.strip() and not line.lstrip().startswith("//")
]
if meaningful_lines != ["pub use openbim_icdd::*;"]:
    fail("icdd library must contain only `pub use openbim_icdd::*;`")

dependencies = alias["dependencies"]
if len(dependencies) != 1:
    fail("icdd must depend only on openbim-icdd")
dependency = dependencies[0]
if dependency["name"] != "openbim-icdd" or dependency.get("rename") is not None:
    fail("icdd's sole dependency must be the unrenamed openbim-icdd package")
if dependency.get("kind") is not None or dependency.get("optional"):
    fail("openbim-icdd must be a required normal dependency")
if dependency.get("target") is not None:
    fail("openbim-icdd dependency must apply on every target")
if dependency.get("features"):
    fail("openbim-icdd dependency must not override canonical features")
if dependency.get("uses_default_features") is not True:
    fail("openbim-icdd dependency must retain canonical default features")
expected_requirement = f"={canonical_version}"
if dependency["req"] != expected_requirement:
    fail(
        f"openbim-icdd requirement must be {expected_requirement}, "
        f"got {dependency['req']}"
    )
expected_dependency_path = normalized(ROOT / "openbim-icdd")
if dependency.get("path") is None:
    fail("openbim-icdd must be a local path dependency for workspace validation")
if normalized(dependency["path"]) != expected_dependency_path:
    fail(
        f"openbim-icdd path must resolve to {expected_dependency_path}, "
        f"got {dependency['path']}"
    )
