---
phase: quick
plan: 1
type: execute
wave: 1
depends_on: []
files_modified:
  - apps/efofx-estimate/.do/app.yaml
autonomous: true
requirements: ["QUICK-1"]

must_haves:
  truths:
    - "VALKEY_URL is declared as a RUN_TIME secret in app.yaml"
    - "RESEND_API_KEY is declared as a RUN_TIME secret in app.yaml"
    - "APP_BASE_URL is declared as a RUN_TIME env var in app.yaml"
    - "MASTER_ENCRYPTION_KEY is declared as a RUN_TIME secret in app.yaml"
  artifacts:
    - path: "apps/efofx-estimate/.do/app.yaml"
      provides: "DigitalOcean App Platform deployment config with all required env vars"
      contains: "VALKEY_URL"
  key_links: []
---

<objective>
Add VALKEY_URL, RESEND_API_KEY, APP_BASE_URL, and MASTER_ENCRYPTION_KEY environment variables to the DigitalOcean app.yaml deployment configuration.

Purpose: These env vars were added in Phases 6-7 (Valkey cache, Resend email, magic links, BYOK encryption) but never added to the DO deployment spec. Production deployment will fail without them.
Output: Updated apps/efofx-estimate/.do/app.yaml with all four env vars properly configured.
</objective>

<execution_context>
@/Users/brettlee/.claude/get-shit-done/workflows/execute-plan.md
@/Users/brettlee/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@apps/efofx-estimate/.do/app.yaml
@apps/efofx-estimate/app/core/config.py
</context>

<tasks>

<task type="auto">
  <name>Task 1: Add missing env vars to app.yaml</name>
  <files>apps/efofx-estimate/.do/app.yaml</files>
  <action>
Add four environment variable entries to the `envs` list in apps/efofx-estimate/.do/app.yaml, in the service `efofx-api`.

Place them in logical groups within the existing env var list:

1. After the "BYOK Encryption" section (ENCRYPTION_KEY, around line 114), add MASTER_ENCRYPTION_KEY as a RUN_TIME SECRET. Note: ENCRYPTION_KEY already exists for per-tenant BYOK; MASTER_ENCRYPTION_KEY is the system-level master key used to wrap/unwrap tenant keys (see byok_service.py). Both are needed.

2. After the "Rate Limiting" section (around line 130), add a new "# Valkey Cache" comment section with:
   - VALKEY_URL as a RUN_TIME SECRET (contains connection credentials; must use rediss:// scheme per Phase 06-01 decision)

3. After the Valkey section, add a new "# Email (Resend)" comment section with:
   - RESEND_API_KEY as a RUN_TIME SECRET

4. After the Email section, add a new "# Application URLs" comment section with:
   - APP_BASE_URL as a RUN_TIME env var with value "https://app.efofx.ai" (used for magic link URLs and auth verification URLs; not a secret)

Each SECRET entry format (no value field — set via DO dashboard):
```yaml
      - key: VARIABLE_NAME
        scope: RUN_TIME
        type: SECRET
```

APP_BASE_URL format (plain value, not secret):
```yaml
      - key: APP_BASE_URL
        value: "https://app.efofx.ai"
        scope: RUN_TIME
```
  </action>
  <verify>grep -c 'VALKEY_URL\|RESEND_API_KEY\|APP_BASE_URL\|MASTER_ENCRYPTION_KEY' apps/efofx-estimate/.do/app.yaml | grep -q '4'</verify>
  <done>All four env vars present in app.yaml with correct scope and type settings. VALKEY_URL, RESEND_API_KEY, and MASTER_ENCRYPTION_KEY are secrets. APP_BASE_URL is a plain value.</done>
</task>

</tasks>

<verification>
- `grep -E 'VALKEY_URL|RESEND_API_KEY|APP_BASE_URL|MASTER_ENCRYPTION_KEY' apps/efofx-estimate/.do/app.yaml` shows all four
- `grep -A2 'VALKEY_URL' apps/efofx-estimate/.do/app.yaml` shows scope: RUN_TIME and type: SECRET
- `grep -A2 'RESEND_API_KEY' apps/efofx-estimate/.do/app.yaml` shows scope: RUN_TIME and type: SECRET
- `grep -A2 'MASTER_ENCRYPTION_KEY' apps/efofx-estimate/.do/app.yaml` shows scope: RUN_TIME and type: SECRET
- `grep -A2 'APP_BASE_URL' apps/efofx-estimate/.do/app.yaml` shows value and scope: RUN_TIME (no type: SECRET)
- YAML is valid (no syntax errors)
</verification>

<success_criteria>
All four environment variables are correctly declared in app.yaml so that DigitalOcean App Platform will prompt for their values during deployment.
</success_criteria>

<output>
After completion, create `.planning/quick/1-add-valkey-url-resend-api-key-app-base-u/1-SUMMARY.md`
</output>
