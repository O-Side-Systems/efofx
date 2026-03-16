---
phase: quick
plan: 1
subsystem: deployment
tags: [app.yaml, digitalocean, env-vars, valkey, resend, byok]
dependency_graph:
  requires: []
  provides: [production-env-vars-complete]
  affects: [apps/efofx-estimate/.do/app.yaml]
tech_stack:
  added: []
  patterns: [digitalocean-app-platform-secrets]
key_files:
  created: []
  modified:
    - apps/efofx-estimate/.do/app.yaml
decisions:
  - APP_BASE_URL uses value (not SECRET) — not a credential, required for magic link URL generation in email templates
metrics:
  duration: ~2min
  completed: 2026-03-15
---

# Quick Task 1: Add VALKEY_URL, RESEND_API_KEY, APP_BASE_URL, MASTER_ENCRYPTION_KEY to app.yaml Summary

**One-liner:** Added four missing production env vars (Valkey cache, Resend email, magic link base URL, BYOK master key) to DigitalOcean App Platform deployment spec so DO prompts for their values on next deployment.

## What Was Done

Four environment variables added to `apps/efofx-estimate/.do/app.yaml` in the `efofx-api` service `envs` list. These vars were introduced in Phases 6-7 but were never reflected in the deployment spec, which would cause production startup failures.

| Variable | Type | Location in file |
|---|---|---|
| MASTER_ENCRYPTION_KEY | RUN_TIME SECRET | After ENCRYPTION_KEY (BYOK section) |
| VALKEY_URL | RUN_TIME SECRET | New "# Valkey Cache" section after Rate Limiting |
| RESEND_API_KEY | RUN_TIME SECRET | New "# Email (Resend)" section after Valkey |
| APP_BASE_URL | RUN_TIME value = "https://app.efofx.ai" | New "# Application URLs" section after Email |

## Verification

All four checks pass:

- `grep -c 'VALKEY_URL\|RESEND_API_KEY\|APP_BASE_URL\|MASTER_ENCRYPTION_KEY' apps/efofx-estimate/.do/app.yaml` returns 4
- VALKEY_URL: scope: RUN_TIME, type: SECRET
- RESEND_API_KEY: scope: RUN_TIME, type: SECRET
- MASTER_ENCRYPTION_KEY: scope: RUN_TIME, type: SECRET
- APP_BASE_URL: value: "https://app.efofx.ai", scope: RUN_TIME (no type: SECRET)

## Commits

| Task | Description | Commit |
|---|---|---|
| 1 | Add four missing env vars to app.yaml | 9506d22 |

## Deviations from Plan

None — plan executed exactly as written.

## Self-Check: PASSED

- [x] `apps/efofx-estimate/.do/app.yaml` modified and committed
- [x] Commit 9506d22 exists
- [x] All four env vars verified present with correct types
