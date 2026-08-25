#!/usr/bin/env python3
"""Mutation-test the semantic alias gate in an isolated workspace copy."""

from __future__ import annotations

import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def copy_candidate(destination: Path) -> None:
    for name in ("Cargo.toml", "Cargo.lock"):
        shutil.copy2(ROOT / name, destination / name)
    for name in ("icdd", "openbim-icdd"):
        shutil.copytree(ROOT / name, destination / name)
    scripts = destination / "scripts"
    scripts.mkdir()
    shutil.copy2(ROOT / "scripts/check_alias_purity.py", scripts)


def run_checker(candidate: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, "scripts/check_alias_purity.py"],
        cwd=candidate,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
    )


def require_rejection(candidate: Path, name: str, mutate: object) -> None:
    alias = candidate / "icdd/Cargo.toml"
    canonical = candidate / "openbim-icdd/Cargo.toml"
    source = candidate / "icdd/src/lib.rs"
    originals = {
        alias: alias.read_text(encoding="utf-8"),
        canonical: canonical.read_text(encoding="utf-8"),
        source: source.read_text(encoding="utf-8"),
    }
    extra = candidate / "icdd/src/owned.rs"
    try:
        mutate(alias, canonical, source, extra)
        result = run_checker(candidate)
        if result.returncode == 0:
            raise SystemExit(f"alias purity mutation escaped: {name}")
    finally:
        for path, content in originals.items():
            path.write_text(content, encoding="utf-8")
        extra.unlink(missing_ok=True)


def replace(path: Path, old: str, new: str) -> None:
    content = path.read_text(encoding="utf-8")
    if old not in content:
        raise SystemExit(f"mutation fixture missing in {path}: {old!r}")
    path.write_text(content.replace(old, new, 1), encoding="utf-8")


def main() -> None:
    with tempfile.TemporaryDirectory(prefix="icdd-alias-purity-") as raw:
        candidate = Path(raw)
        copy_candidate(candidate)
        if run_checker(candidate).returncode != 0:
            raise SystemExit("baseline alias purity checker failed")

        require_rejection(
            candidate,
            "target-gated dependency",
            lambda alias, _canonical, _source, _extra: replace(
                alias, "[dependencies]", "[target.'cfg(unix)'.dependencies]"
            ),
        )
        require_rejection(
            candidate,
            "disabled default features",
            lambda alias, _canonical, _source, _extra: replace(
                alias,
                'path = "../openbim-icdd"',
                'path = "../openbim-icdd", default-features = false',
            ),
        )

        def add_feature(alias: Path, canonical: Path, _source: Path, _extra: Path) -> None:
            canonical.write_text(
                canonical.read_text(encoding="utf-8") + "\n[features]\nprobe = []\n",
                encoding="utf-8",
            )
            replace(
                alias,
                'path = "../openbim-icdd"',
                'path = "../openbim-icdd", features = ["probe"]',
            )

        require_rejection(candidate, "dependency feature override", add_feature)

        def alternate_target(alias: Path, _canonical: Path, _source: Path, extra: Path) -> None:
            extra.write_text("pub struct Owned;\n", encoding="utf-8")
            alias.write_text(
                alias.read_text(encoding="utf-8") + '\n[lib]\npath = "src/owned.rs"\n',
                encoding="utf-8",
            )

        require_rejection(candidate, "alternate active library target", alternate_target)
        require_rejection(
            candidate,
            "unreferenced implementation file",
            lambda _alias, _canonical, _source, extra: extra.write_text(
                "pub struct HiddenOwned;\n", encoding="utf-8"
            ),
        )
        require_rejection(
            candidate,
            "owned source item",
            lambda _alias, _canonical, source, _extra: source.write_text(
                "pub use openbim_icdd::*;\npub struct Owned;\n", encoding="utf-8"
            ),
        )
        require_rejection(
            candidate,
            "alias version drift with decoy",
            lambda alias, _canonical, _source, _extra: (
                replace(alias, 'version = "0.1.0"', 'version = "0.1.1"'),
                alias.write_text(
                    'version = "0.1.0"\n' + alias.read_text(encoding="utf-8"),
                    encoding="utf-8",
                ),
            ),
        )

    print("alias purity mutations passed")


if __name__ == "__main__":
    main()
