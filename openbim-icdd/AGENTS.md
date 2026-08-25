# openbim-icdd instructions

Purpose: canonical ISO 21597 Information Container for linked Document Delivery
implementation. This crate owns every ICDD type and behavior.

Follow `../AGENTS.md`. Read `PLAN.md` for assigned implementation or roadmap
work; keep progress, blockers, and evidence there.

## Boundary

May consume `openbim-core`, ZIP framing, and public IFC contracts. RDF remains
inside this crate until another real consumer exists. Payload documents remain
opaque unless callers explicitly parse them.

## Status

Reserved scaffold. Do not claim parsing, writing, or validation yet.
