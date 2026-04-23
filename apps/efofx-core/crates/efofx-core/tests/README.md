# efofx-core integration tests

## What's covered

| Layer                        | Coverage                | Location                             |
|------------------------------|-------------------------|--------------------------------------|
| JWT + JWKS auth middleware   | ✅ End-to-end            | `tests/auth_contract.rs`             |
| Tenant DTO ↔ domain mapping  | ✅ Unit                  | `efofx-storage/src/chat.rs` tests    |
| Scoping extractor            | ✅ Unit (31 cases)       | `efofx-domain/src/scoping.rs`        |
| Prompt registry              | ✅ Unit + shipped JSONs  | `efofx-prompts/tests/`               |
| LLM provider + classification| ✅ Unit                  | `efofx-llm/src/{error,mock}.rs`      |
| Chat budget checks           | ✅ Pure-function unit    | `efofx-core/src/services/chat.rs`    |
| Chat handler error mapping   | ✅ Unit                  | `efofx-core/src/services/chat.rs`    |

## What's not covered — **integration gap**

Phase 2B ships *without* end-to-end tests that exercise the full
`middleware → handler → ChatService → ChatRepo → MongoDB` path against a
real database. That means the following wiring is only validated by a
live deploy or a manual curl:

- `ensure_indexes` actually creates the unique and TTL indexes on `chat_sessions`.
- `$push` / `$inc` / aggregation queries in `ChatRepo` behave as expected against
  a real Mongo server.
- `TenantContext` minted by the auth middleware propagates intact into
  `ChatService::append_message` and back into repo calls.
- The happy-path flow (create session → append user → LLM follow-up → persist
  assistant → state transition to `ready`) completes with the right side
  effects persisted.

This mirrors the Phase 2A gap for `/v1/me` and BYOK endpoints — the
pattern is consistent; the gap isn't new to 2B.

## Closing the gap

Two known paths:

1. **testcontainers-mongo** — adds a `testcontainers` dev-dependency,
   spins up a MongoDB docker container per test module. Requires Docker
   in CI. Estimated work: 2–3 hours for a first test, ~1 hour per
   additional scenario.

2. **Staging smoke tests** — a checked-in bash/curl script or Rust binary
   that hits a deployed Rust server with realistic payloads. Cheaper
   upfront, but catches regressions only after deploy.

Recommendation: testcontainers for the critical paths (create-session +
append happy path + budget ceilings + expiry), smoke tests for the
cross-surface flows (widget → chat → estimate SSE). Both can live in a
dedicated `efofx-core/tests/integration/` module once added.

Tracked as a Phase 4 (Hardening) work item — not a blocker for the
demo-path phases.
