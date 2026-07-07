# efofx-core

Rust platform core for efofx. See:

- [`docs/efofx_platform_core_tadr.md`](../../docs/efofx_platform_core_tadr.md) — architecture decisions
- [`docs/RUST-PORT-PLAN.md`](../../docs/RUST-PORT-PLAN.md) — phased migration plan

## Workspace layout

```
apps/efofx-core/
├── Cargo.toml                  # workspace manifest
├── config/efofx-core.toml      # default config (no secrets)
├── .env.example                # env override template
└── crates/
    ├── efofx-core/             # Axum server binary
    ├── efofx-domain/           # pure domain types (no I/O)
    ├── efofx-config/           # layered config loader
    ├── efofx-storage/          # Mongo adapter + TenantContext guard
    └── efofx-openapi/          # OpenAPI doc assembly + response envelopes
```

## Local development

```
cd apps/efofx-core
cp .env.example .env
# edit .env — at minimum set EFOFX_MONGO__URI
cargo run -p efofx-core
```

The server listens on `127.0.0.1:8080` by default. Useful endpoints:

- `GET /health` — liveness + Mongo reachability
- `GET /openapi.json` — OpenAPI 3 document for this service

## Commands

```
cargo build                 # compile all crates
cargo test                  # run unit + integration tests
cargo clippy -- -D warnings # lint, treat warnings as errors
cargo fmt                   # format
```

## Phase status

Phase 3 — Client cutover & FastAPI decommission (complete, 2026-04-28).
This crate is the authoritative backend; both the widget and the
dashboard call it directly. See `docs/CODEBASE-STATE.md` for the living
state document and `docs/rust-port/` for phase plans and checkpoints.

Known Phase 4 deferrals: dashboard lead endpoints (`/v1/leads*`) return
`501`, `EstimationOutput` is not yet persisted alongside the session
(`GET /v1/estimates/{id}` returns `result: null`), and settings written
via `PATCH /v1/me` are not yet echoed back on `GET /v1/me`.

## Operational notes

- Request bodies are capped by axum's default `DefaultBodyLimit`
  (2 MB). No endpoint needs more; revisit if that changes.
- Rate limiting covers the public widget surface (per-IP). Authenticated
  chat/estimation endpoints rely on tenant auth + BYOK cost ownership;
  per-tenant quotas are a Phase 4 item.
