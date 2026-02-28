---
phase: 03-llm-integration
plan: 02
subsystem: prompt-registry
tags: [prompts, versioning, immutability, registry, traceability]
requirements: [PRMT-01, PRMT-02, PRMT-03]

dependency_graph:
  requires:
    - 03-01 (LLMService — context on generate_estimation system prompt to migrate)
  provides:
    - PromptService class-level registry (load_all, get, get_version_string, list_versions, clear)
    - Three v1.0.0 prompt JSON files (scoping, narrative, estimation)
    - EstimationSession.prompt_version field for traceability
    - App lifespan startup wires PromptService.load_all()
  affects:
    - 03-03 (conversation engine — uses PromptService.get("scoping", "latest"))
    - 03-04 (narrative generator — uses PromptService.get("narrative", "latest"))

tech_stack:
  added:
    - pathlib.Path.glob (JSON file discovery)
    - hashlib.sha256 (content-hash immutability enforcement)
    - Simple semver tuple sort (no packaging dependency)
  patterns:
    - Class-level registry singleton (PromptService._registry, PromptService._content_hashes)
    - SHA-256 content hashing for immutability detection at load time
    - "latest" version resolution via semver tuple comparison
    - Optional[str] backward-compatible field addition to Pydantic model

key_files:
  created:
    - apps/efofx-estimate/app/services/prompt_service.py
    - apps/efofx-estimate/config/prompts/v1.0.0-scoping.json
    - apps/efofx-estimate/config/prompts/v1.0.0-narrative.json
    - apps/efofx-estimate/config/prompts/v1.0.0-estimation.json
    - apps/efofx-estimate/tests/services/test_prompt_service.py
  modified:
    - apps/efofx-estimate/app/models/estimation.py
    - apps/efofx-estimate/app/main.py

decisions:
  - "PromptService uses class-level _registry dict (not instance) — single shared registry loaded once at app startup, no DI complexity"
  - "SHA-256 content hash stored at load time — immutability check compares hash on second load, not field-by-field diff"
  - "prompt_version is Optional[str] = None on EstimationSession — backward-compatible with existing MongoDB documents that predate this field"
  - "clear() classmethod provided for test isolation — resets both _registry and _content_hashes"
  - "prompts_dir uses os.path.abspath(__file__) in main.py — resolves correctly regardless of CWD at startup"
  - "Startup raises on PromptService failure — prompts are critical, fail fast rather than silently serve requests without prompt registry"

metrics:
  duration: 3 min
  completed_date: "2026-02-27"
  tasks_completed: 2
  tasks_total: 2
  files_modified: 7
---

# Phase 3 Plan 02: Prompt Registry Summary

**One-liner:** Git-versioned prompt registry with SHA-256 immutability enforcement, semver "latest" resolution, and three v1.0.0 prompt files (scoping, narrative, estimation) loaded at app startup.

## What Was Built

### Task 1: PromptService and versioned prompt JSON files

**`app/services/prompt_service.py`** — new module implementing the prompt registry:

- `PromptService._registry: dict[str, dict]` — class-level dict keyed by `"{name}:{version}"`. Stores full prompt dicts from JSON files.
- `PromptService._content_hashes: dict[str, str]` — class-level dict keyed by `"{name}:{version}"`. Stores SHA-256 hex digest of each file's raw content.
- `load_all(cls, prompts_dir: str) -> None` — classmethod. Iterates `*.json` files via `pathlib.Path.glob()`. Validates required fields (`version`, `name`, `created_at`, `system_prompt`, `user_prompt_template`). Raises `ValueError` on missing fields or immutability violations (existing key + different hash). Idempotent for same content (same hash = skip silently).
- `get(cls, name: str, version: str = "latest") -> dict` — classmethod. `"latest"` resolves to highest semver via `tuple(int(x) for x in v.split("."))` comparison. Raises `KeyError` for missing prompts.
- `get_version_string(cls, name: str, version: str = "latest") -> str` — convenience for `prompt_version` field population.
- `list_versions(cls, name: str) -> list[str]` — returns semver-sorted list of versions for a name.
- `clear(cls) -> None` — resets both dicts for test isolation.

**Three v1.0.0 prompt JSON files** in `config/prompts/`:

- `v1.0.0-scoping.json` (name: "scoping") — Conversational intake prompt. Instructs LLM to ask one focused question at a time, gather project type/size/location/timeline, signal `READY_FOR_ESTIMATE` when all four are present.
- `v1.0.0-narrative.json` (name: "narrative") — Plain-language estimate narrative. Instructs LLM to lead with P50/P80 bottom line using "most likely"/"budget for" language, avoid statistical jargon.
- `v1.0.0-estimation.json` (name: "estimation") — Structured cost estimation. Migrated from `llm_service.py generate_estimation()` system prompt.

**`tests/services/test_prompt_service.py`** — 14 tests:
- Load all prompts from directory
- Get by exact name + version
- Get latest returns highest semver
- Get nonexistent raises KeyError (both exact and latest)
- Immutability violation raises ValueError
- get_version_string returns correct version string (latest + explicit)
- list_versions returns sorted ascending list + excludes other names
- clear() resets registry
- Missing required fields raises ValueError
- Idempotent load (same content, no-op)
- Real prompt files smoke test

All 14 tests pass.

### Task 2: Add prompt_version to EstimationSession and wire PromptService into app lifespan

**`app/models/estimation.py`**:
- `prompt_version: Optional[str] = Field(None, description="Prompt version used for this estimate (e.g., '1.0.0')")` added to `EstimationSession` after `confidence_threshold`
- `schema_extra` example updated with `"prompt_version": "1.0.0"`
- `Optional[str] = None` ensures backward compatibility with existing documents

**`app/main.py`**:
- `from app.services.prompt_service import PromptService` imported
- `lifespan()` updated to call `PromptService.load_all(prompts_dir)` after MongoDB connection
- `prompts_dir` resolved with `os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "config", "prompts")` for reliable path resolution
- Raises on failure — prompts are critical, startup fails if registry can't load

## Verification

- `python -m pytest tests/services/test_prompt_service.py -x -v` — 14/14 passed
- `python -m pytest tests/ --ignore=tests/api -m "not integration" -k "not test_performance_requirement"` — 138/138 passed
- All three prompt files exist in `config/prompts/` with valid schema
- `PromptService.get("scoping", "latest")` returns v1.0.0 scoping prompt
- `EstimationSession` model accepts `prompt_version` field
- Immutability test: modifying file content and reloading raises `ValueError: Immutability violation`
- API tests fail due to pre-existing Redis connection issue (localhost:6379 not running) — unrelated to this plan, documented in 03-01-SUMMARY.md

## Deviations from Plan

None — plan executed exactly as written.

## Commits

| Hash | Message |
|------|---------|
| ff28bca | feat(03-02): create PromptService and versioned prompt JSON files |
| e5acaee | feat(03-02): add prompt_version to EstimationSession and wire PromptService into app lifespan |

## Self-Check: PASSED

- FOUND: apps/efofx-estimate/app/services/prompt_service.py
- FOUND: apps/efofx-estimate/config/prompts/v1.0.0-scoping.json
- FOUND: apps/efofx-estimate/config/prompts/v1.0.0-narrative.json
- FOUND: apps/efofx-estimate/config/prompts/v1.0.0-estimation.json
- FOUND: apps/efofx-estimate/tests/services/test_prompt_service.py
- FOUND: apps/efofx-estimate/app/models/estimation.py (modified)
- FOUND: apps/efofx-estimate/app/main.py (modified)
- FOUND: commit ff28bca (PromptService + prompt JSON files)
- FOUND: commit e5acaee (EstimationSession prompt_version + lifespan wiring)
