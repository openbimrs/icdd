# Contributing

Contributions are welcome, especially those that turn the reserved ICDD
contracts into lossless, conformance-tested container behavior.

## Before opening a pull request

1. Read `AGENTS.md` and the affected crate's nested instructions.
2. Put implementation only in `openbim-icdd`; keep `icdd` a pure re-export.
3. Preserve payload bytes and unknown RDF data unless an explicit API contract
   says otherwise.
4. Add tests before claiming parsing, writing, validation, or migration behavior.
5. Use public, redistributable fixtures. Do not commit restricted standards.
6. Run:

```bash
./scripts/gate.sh
```

7. Update README capability status, rustdoc, and `CHANGELOG.md` when behavior is
   user-visible.

## Commits

Use focused commits with imperative subjects. Cross-repository changes publish
the canonical package before the exact-version alias and update the
`openbimrs/openbim` submodule pin last.
