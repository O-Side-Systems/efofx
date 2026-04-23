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

Phase 0 — Foundations (in progress).
