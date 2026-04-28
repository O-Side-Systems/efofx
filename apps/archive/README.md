# Archived apps

This directory holds non-authoritative apps kept around for historical
reference only. The live backend is **`apps/efofx-core`** (Rust).

| Path | Original role | Status |
|---|---|---|
| `efofx-estimate-fastapi/` | Original FastAPI backend | Decommissioned 2026-04-28 (rust-port Phase 3.5). Superseded by `apps/efofx-core`. |

Nothing in here is built, tested, or deployed. Treat the contents as a
read-only reference for PRD history, calibration prompt evolution, and
domain context that wasn't carried forward verbatim into the Rust port.

If a new feature needs something from here, port it into `efofx-core`
first; do not revive the archive.
