# LLM Prompts Directory

This directory contains git-versioned JSON prompt files for the efOfX Estimation Service.

## Architecture Decision

Per `docs/architecture.md`, this project uses **Git-Based JSON Files** for prompt management:

- ✅ Zero dependencies
- ✅ Git version control (diff/blame/PR review)
- ✅ Store `prompt_version` in estimates for calibration tracking
- ✅ Upgrade path to LangSmith post-MVP

## Prompt File Structure

```json
{
  "version": "1.0.0",
  "prompt_id": "estimate_generation",
  "prompt_text": "...",
  "metadata": {
    "created_at": "2025-11-10",
    "updated_at": "2025-11-10",
    "author": "team"
  }
}
```

## Usage

Prompt files are loaded by the LLM integration service and versioned with each estimate for calibration feedback loop.
