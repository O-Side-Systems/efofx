# `seed-references` — operational runbook

One-shot binary that loads **platform-scoped** reference classes and
reference projects (`tenant_id: null`) into MongoDB. Tenant-scoped rows
are never touched.

Source: `apps/efofx-core/crates/efofx-core/src/bin/seed-references.rs`
Fixtures: `apps/efofx-core/config/reference-data/*.json`

---

## When to run

- **First-time setup** of any efofx-core environment (local dev,
  staging, prod) before a chat session can generate a useful estimate —
  the estimation service pulls reference projects as grounding data.
- **After editing fixtures** — the binary is idempotent by
  `(tenant_id: null, name)` for classes and `(tenant_id: null, project_id)`
  for projects, so re-running just reconciles changes.
- **After schema changes** that render existing rows stale — run with
  `--clean` to wipe platform rows first.

Not needed on every deploy. Intended to be invoked manually by an
operator, not by a startup hook.

---

## Prerequisites

- MongoDB reachable from the machine running the binary
- Same environment variables the server needs:
  - `EFOFX_MONGO__URI` (required, treat as secret)
  - `EFOFX_SUPABASE__URL` (required by config validation, not contacted)
  - `EFOFX_SUPABASE__ISSUER` (required by config validation, not contacted)
- Optional: `EFOFX_MONGO__DB_NAME` if overriding from `efofx-core.toml`

## Commands

Run from `apps/efofx-core/` (so the default `config/reference-data`
path resolves). Substitute the path if running from elsewhere via
`--dir`.

```bash
# Validate fixtures, report totals, no writes. Always run this first.
cargo run -p efofx-core --bin seed-references -- --dry-run

# Idempotent upsert (default). Safe to run repeatedly.
cargo run -p efofx-core --bin seed-references

# Wipe all platform rows first, then upsert. Use when a schema or
# fixture rename would otherwise leave orphan rows.
cargo run -p efofx-core --bin seed-references -- --clean

# Load fixtures from a different directory (e.g. in CI).
cargo run -p efofx-core --bin seed-references -- --dir ops/fixtures
```

Release builds live under
`target/release/seed-references` after `cargo build --release --bin seed-references`.

## What the binary does, step by step

1. Parses CLI flags (`--dry-run`, `--clean`, `--dir <path>`).
2. Loads `AppConfig` — same layered loader (`config/efofx-core.toml`
   → `EFOFX_*` env) as the server, so Mongo URI comes from the
   environment.
3. Reads every `*.json` file under the fixture directory (sorted, for
   deterministic logs).
4. Validates:
   - class names are unique across all bundles
   - project IDs are unique across all bundles
   - `cost_breakdown_template` values sum to ~1.0 (within 1% tolerance,
     matching `ReferenceClass::validate_cost_breakdown`)
   - every project's `cost_breakdown` key is present in its class'
     `cost_breakdown_template`
   - `quality_score` is within `[0, 1]`
5. Converts fixtures to domain values, stamping `tenant_id: null`,
   `created_at: now`, `is_active: true`.
6. If `--dry-run`: reports totals and exits.
7. Connects to Mongo, calls `ReferenceRepo::ensure_indexes` (idempotent).
8. If `--clean`: deletes all `tenant_id: null` rows in both collections.
9. Upserts each class via `replace_one({tenant_id: null, name: ...},
   class, upsert: true)` and each project via `replace_one({tenant_id:
   null, project_id: ...}, project, upsert: true)`.
10. Emits a summary log with inserted/modified/unchanged counts.

## Fixture format

Each JSON file under `config/reference-data/` is a bundle: one class plus
its seed projects.

```jsonc
{
  "class": {
    "category": "construction",
    "subcategory": "pool",
    "name": "residential_pool_socal",
    "description": "…",
    "keywords": ["…"],
    "regions": ["SoCal - Coastal"],
    "attributes": { "size_range": "300-500 sq ft" },
    "cost_distribution": { "p50": 55000, "p80": 72000, "p95": 95000, "currency": "USD" },
    "timeline_distribution": { "p50_days": 45, "p80_days": 60, "p95_days": 90 },
    "cost_breakdown_template": {
      "excavation": 0.15,
      "materials":  0.40,
      "labor":      0.30,
      "permits":    0.05,
      "overhead":   0.10
    },
    "is_synthetic": true,
    "validation_source": "synthetic_seed_v1"
  },
  "projects": [
    {
      "project_id": "pool_socal_001",
      "region": "SoCal - Coastal",
      "description": "…",
      "size_sqft": 450.0,
      "total_cost": 58500.0,
      "timeline_weeks": 8,
      "team_size": 4,
      "cost_breakdown": { "excavation": 8775, "materials": 23400, "labor": 17550, "permits": 2925, "overhead": 5850 },
      "completion_date": "2025-06-15T00:00:00Z",
      "quality_score": 0.9,
      "source": "synthetic_seed"
    }
  ]
}
```

Fields the binary auto-fills (do **not** include them in fixtures):
- `tenant_id` → always `null` for seeded rows
- project `reference_class` → inherited from the bundle's class name
- `is_active` on projects → always `true`
- `created_at` → stamped with the time of the run

## Adding a new class

1. Copy an existing bundle (`construction-pool.json` is the shortest).
2. Pick a unique `class.name`. Snake_case, optionally domain-suffixed
   (e.g. `residential_pool_socal`). This becomes the join key against
   project rows.
3. Pick unique `project_id` values. Prefix with the class shorthand for
   readability (`pool_socal_001`).
4. Make sure `cost_breakdown_template` percentages sum to 1.0 and every
   project's `cost_breakdown` uses the same keys.
5. Each project's `cost_breakdown` should sum to that project's
   `total_cost`. The validator doesn't enforce this yet (future: add a
   soft check), but downstream analysis assumes it.
6. Run `--dry-run` to validate, then the normal command to apply.

## Idempotence semantics

- Re-running without `--clean` only writes rows whose key already exists
  (a replace) or is missing (an insert). The log reports counts of
  `classes_inserted` / `classes_modified` / `classes_unchanged` and the
  same trio for projects.
- Removing a class or project from a fixture does **not** delete its
  row on the next run — this is by design, to protect against
  accidental fixture-file deletion wiping production data. Use
  `--clean` when you deliberately want to drop old rows.
- Changing a class `name` or project `project_id` creates a new row
  rather than renaming. Clean up the old one by hand, or use `--clean`.

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `configuration validation failed: mongo.uri is required` | `EFOFX_MONGO__URI` not set | Export it (e.g. `set -a; source .env; set +a`) |
| `configuration validation failed: supabase.url is required` | Running without the same env the server uses | Set `EFOFX_SUPABASE__URL` / `EFOFX_SUPABASE__ISSUER` (values aren't contacted — any valid URL works) |
| `fixture directory does not exist` | Wrong cwd or `--dir` typo | Run from `apps/efofx-core/` or pass an absolute `--dir` |
| `duplicate class name` / `duplicate project_id` | Two fixture bundles collide | Rename one; IDs are global across the directory |
| `cost_breakdown_template sums to X, expected ~1.0` | Percentages don't sum to 1.0 ±0.01 | Recheck math; use the tolerance breathing room only for genuine rounding |
| Mongo `ServerSelectionTimeoutError` on connect | Wrong URI / network reachability | Verify `mongosh "$EFOFX_MONGO__URI"` works from the same host |

## See also

- `docs/rust-port/phase-2c-checkpoint.md` — Phase 2C context
- `docs/RUST-PORT-PLAN.md` §Phase 2C — plan and DoD
- `apps/efofx-core/crates/efofx-domain/src/reference.rs` — domain schema
- `apps/efofx-core/crates/efofx-storage/src/reference.rs` — repository
