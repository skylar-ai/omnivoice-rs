# Project state

## Decisions

| ID | Decision | Rationale | Status | Date |
| --- | --- | --- | --- | --- |
| AD-001 | New generation knobs are plain `pub` fields on `GenerationConfig` with a `Default` that keeps upstream Python parity; any field added bumps the `omnivoice-infer` skylar prerelease | User asked "how should we handle the public API?" and chose "Add fields + bump"; parity defaults keep the reference tests green | active | 2026-11-16 |
| AD-002 | Server and CLI time parameters are in seconds, `snake_case` on the wire with `kebab-case` and `snake_case` CLI aliases | Matches `duration` and `audio_chunk_duration`; clients bind to these names | active | 2026-11-16 |
