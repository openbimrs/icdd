#!/usr/bin/env python3
"""Package both crates and compile the alias against this candidate."""

from __future__ import annotations

import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile

ROOT = Path(__file__).resolve().parents[1]
TARGET = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
CANONICAL = "openbim-icdd"
ALIAS = "icdd"


def run(
    *command: str, cwd: Path = ROOT, capture: bool = False
) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        command,
        cwd=cwd,
        check=False,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE if capture else None,
    )
    if result.returncode != 0:
        if result.stdout:
            print(result.stdout, file=sys.stderr)
        if result.stderr:
            print(result.stderr, file=sys.stderr)
        raise SystemExit(result.returncode)
    return result


metadata = json.loads(
    run(
        "cargo",
        "metadata",
        "--locked",
        "--no-deps",
        "--format-version",
        "1",
        capture=True,
    ).stdout
)
versions = {package["name"]: package["version"] for package in metadata["packages"]}
canonical_version = versions[CANONICAL]
alias_version = versions[ALIAS]
if canonical_version != alias_version:
    raise SystemExit("canonical and alias package versions differ")

run("cargo", "package", "--locked", "-p", CANONICAL)
run(
    "cargo",
    "package",
    "--locked",
    "--no-verify",
    "--config",
    f'patch.crates-io.{CANONICAL}.path="{(ROOT / CANONICAL).as_posix()}"',
    "-p",
    ALIAS,
)

package_root = TARGET / "package"
canonical_source = package_root / f"{CANONICAL}-{canonical_version}"
alias_source = package_root / f"{ALIAS}-{alias_version}"
if alias_source.exists():
    shutil.rmtree(alias_source)
for crate_name, version, source in (
    (CANONICAL, canonical_version, canonical_source),
    (ALIAS, alias_version, alias_source),
):
    if not source.is_dir():
        archive = package_root / f"{crate_name}-{version}.crate"
        if not archive.is_file():
            raise SystemExit(f"Cargo package archive missing: {archive}")
        with tarfile.open(archive, "r:gz") as package:
            package.extractall(package_root, filter="data")
    if not source.is_dir():
        raise SystemExit(f"Cargo package extraction missing: {source}")

config_dir = alias_source / ".cargo"
config_dir.mkdir(exist_ok=True)
(config_dir / "config.toml").write_text(
    "[patch.crates-io]\n"
    f'{CANONICAL} = {{ path = "{canonical_source.as_posix()}" }}\n',
    encoding="utf-8",
)
run(
    "cargo",
    "test",
    "--manifest-path",
    str(alias_source / "Cargo.toml"),
    cwd=alias_source,
)

resolved = json.loads(
    run(
        "cargo",
        "metadata",
        "--manifest-path",
        str(alias_source / "Cargo.toml"),
        "--format-version",
        "1",
        cwd=alias_source,
        capture=True,
    ).stdout
)
resolved_canonical = next(
    package for package in resolved["packages"] if package["name"] == CANONICAL
)
if Path(resolved_canonical["manifest_path"]).resolve() != (
    canonical_source / "Cargo.toml"
).resolve():
    raise SystemExit("packaged alias did not resolve the candidate canonical package")

print(
    f"package verification passed: {ALIAS} {alias_version} compiled against "
    f"candidate {CANONICAL} {canonical_version}"
)
