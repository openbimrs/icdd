# OpenBIM.rs ICDD

Canonical repository: <https://github.com/openbimrs/icdd>
Integration repository: <https://github.com/openbimrs/openbim>

Read `AGENTS.md` before changing the repository and the nested `AGENTS.md`
before editing a crate. Keep both packages independently buildable; the parent
OpenBIM.rs workspace pins this repository as a submodule but is not required for
standalone development.

## Verification

Run `./scripts/gate.sh`. It is the authoritative local and CI gate and decides
success from command exit codes.

## Project conventions

- Rust 2021, MSRV 1.88, Python 3.10+ for gate scripts, MIT.
- Pure Rust; unsafe code is forbidden.
- `openbim-icdd` owns every implementation and type.
- `icdd` is a pure, exact-version re-export alias and defines no types.
- ICDD consumes core/codec/IFC contracts; those layers never depend on ICDD.
- Never commit standards PDFs, schemas, or other artifacts without confirmed
  redistribution rights. Local material belongs under ignored `references/`.
- Use Keep a Changelog and distinguish implemented from reserved capabilities.
