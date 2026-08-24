# icdd implementation plan

Status: name reserved; implementation not started.
Last updated: 2026-08-24

This is task state, not ambient context. Follow `AGENTS.md`; claim one task ID,
record blockers/decisions under it, and check it off only with evidence.

## Established boundary

Exactly one line of code: pub use <canonical>::*. Defining a type here is a defect -- see scripts/check-alias-purity.sh.

## Open work

See `docs/ROADMAP.md` Stage 5 for sequencing. Nothing is claimed here yet.

## Planned file map

These paths are compiled private scaffold modules. Implement inside the named
owner and expose a public symbol only through an intentional parent re-export.

- (none claimed yet)

## Work queue

- [ ] `ALI-ICDD` - keep this a pure re-export; nothing to implement

## Completion log

Nothing completed yet. Record the proof command and its result here when an
item above is checked off.
