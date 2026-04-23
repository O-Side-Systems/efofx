# Prompt Registry

Versioned, immutable prompt files loaded by `efofx-prompts` at server startup.

## File format

Each prompt lives in its own JSON file:

```json
{
  "name": "scoping",
  "version": "1.0.0",
  "created_at": "2026-02-27",
  "description": "Conversational scoping prompt ...",
  "system_prompt": "You are ...",
  "user_prompt_template": "Conversation so far: {conversation_history} ..."
}
```

Required fields: `name`, `version`, `created_at`, `system_prompt`, `user_prompt_template`.
Optional: `description`.

Filenames are descriptive only (`v1.0.0-scoping.json`). The `(name, version)` pair inside the JSON is authoritative.

## Placeholder syntax

Templates use `{key}` substitution. No escapes; no format specs. The renderer
errors if a template references a placeholder the caller did not supply.

Numeric values **must be pre-formatted by the caller** — unlike the Python
side's `{total_cost_p50:,.0f}`, the Rust renderer treats a format spec as
part of the placeholder name and will not find it.

## Immutability

Versions are locked once loaded. If two files on disk claim the same
`{name}:{version}` pair but hash differently, startup fails with an
immutability violation. To change a prompt, bump the version.

## Current versions

| Name        | Version | Notes |
|-------------|---------|-------|
| `scoping`   | 1.0.0   | Verbatim port from `apps/efofx-estimate/config/prompts/v1.0.0-scoping.json`. |
| `estimation`| 1.0.0   | Verbatim port. |
| `narrative` | 1.1.0   | Bumped from FastAPI's 1.0.0 because numeric placeholders no longer carry Python format specs; callers pre-format values into display strings. |
