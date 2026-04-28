"""
Tests for PromptService — versioned, immutable prompt registry.

Uses tmp_path pytest fixture for temporary directories with test prompt files.
PromptService.clear() is called before each test to ensure isolation.
"""

import json
import pytest

from app.services.prompt_service import PromptService


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_prompt_file(tmp_path, name: str, version: str, extra: dict = None) -> None:
    """Write a minimal valid prompt JSON file to tmp_path."""
    data = {
        "version": version,
        "name": name,
        "created_at": "2026-02-27",
        "description": f"Test prompt for {name}",
        "system_prompt": f"System prompt for {name} v{version}",
        "user_prompt_template": "Hello {user_message}",
    }
    if extra:
        data.update(extra)
    filename = f"v{version}-{name}.json"
    (tmp_path / filename).write_text(json.dumps(data), encoding="utf-8")


# ---------------------------------------------------------------------------
# Fixture: clear registry before each test
# ---------------------------------------------------------------------------


@pytest.fixture(autouse=True)
def clear_registry():
    """Ensure a clean registry before each test."""
    PromptService.clear()
    yield
    PromptService.clear()


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


def test_load_all_loads_prompts(tmp_path):
    """load_all should load all JSON files from a directory."""
    make_prompt_file(tmp_path, "scoping", "1.0.0")
    make_prompt_file(tmp_path, "narrative", "1.0.0")

    PromptService.load_all(str(tmp_path))

    # Both prompts should be retrievable
    scoping = PromptService.get("scoping", "1.0.0")
    narrative = PromptService.get("narrative", "1.0.0")

    assert scoping["name"] == "scoping"
    assert narrative["name"] == "narrative"


def test_get_by_name_and_version(tmp_path):
    """get() with explicit version returns the correct prompt dict."""
    make_prompt_file(tmp_path, "scoping", "1.0.0")
    PromptService.load_all(str(tmp_path))

    result = PromptService.get("scoping", "1.0.0")

    assert result["version"] == "1.0.0"
    assert result["name"] == "scoping"
    assert "System prompt for scoping v1.0.0" in result["system_prompt"]


def test_get_latest_returns_highest_version(tmp_path):
    """get('name', 'latest') should return the highest semver version."""
    make_prompt_file(tmp_path, "scoping", "1.0.0")
    make_prompt_file(tmp_path, "scoping", "1.1.0")
    PromptService.load_all(str(tmp_path))

    result = PromptService.get("scoping", "latest")

    assert result["version"] == "1.1.0"


def test_get_nonexistent_raises_keyerror(tmp_path):
    """get() for an unregistered prompt raises KeyError."""
    make_prompt_file(tmp_path, "scoping", "1.0.0")
    PromptService.load_all(str(tmp_path))

    with pytest.raises(KeyError, match="nonexistent"):
        PromptService.get("nonexistent", "1.0.0")


def test_get_latest_nonexistent_raises_keyerror(tmp_path):
    """get('name', 'latest') for an unregistered prompt raises KeyError."""
    PromptService.load_all(str(tmp_path))  # empty directory

    with pytest.raises(KeyError, match="ghost"):
        PromptService.get("ghost", "latest")


def test_immutability_violation_raises(tmp_path):
    """Reloading a prompt with changed content should raise ValueError."""
    make_prompt_file(tmp_path, "scoping", "1.0.0")
    PromptService.load_all(str(tmp_path))

    # Modify the file content
    modified_data = {
        "version": "1.0.0",
        "name": "scoping",
        "created_at": "2026-02-27",
        "description": "Modified description",
        "system_prompt": "CHANGED system prompt",
        "user_prompt_template": "Hello {user_message}",
    }
    (tmp_path / "v1.0.0-scoping.json").write_text(
        json.dumps(modified_data), encoding="utf-8"
    )

    with pytest.raises(ValueError, match="Immutability violation"):
        PromptService.load_all(str(tmp_path))


def test_get_version_string(tmp_path):
    """get_version_string should return just the version string."""
    make_prompt_file(tmp_path, "narrative", "1.0.0")
    PromptService.load_all(str(tmp_path))

    version = PromptService.get_version_string("narrative", "latest")

    assert version == "1.0.0"


def test_get_version_string_explicit(tmp_path):
    """get_version_string with explicit version returns that version string."""
    make_prompt_file(tmp_path, "narrative", "1.0.0")
    PromptService.load_all(str(tmp_path))

    version = PromptService.get_version_string("narrative", "1.0.0")

    assert version == "1.0.0"


def test_list_versions(tmp_path):
    """list_versions should return all versions sorted ascending."""
    make_prompt_file(tmp_path, "scoping", "1.1.0")
    make_prompt_file(tmp_path, "scoping", "1.0.0")
    make_prompt_file(tmp_path, "scoping", "2.0.0")
    PromptService.load_all(str(tmp_path))

    versions = PromptService.list_versions("scoping")

    assert versions == ["1.0.0", "1.1.0", "2.0.0"]


def test_list_versions_excludes_other_names(tmp_path):
    """list_versions should only return versions for the given name."""
    make_prompt_file(tmp_path, "scoping", "1.0.0")
    make_prompt_file(tmp_path, "narrative", "1.0.0")
    make_prompt_file(tmp_path, "narrative", "1.1.0")
    PromptService.load_all(str(tmp_path))

    scoping_versions = PromptService.list_versions("scoping")
    narrative_versions = PromptService.list_versions("narrative")

    assert scoping_versions == ["1.0.0"]
    assert narrative_versions == ["1.0.0", "1.1.0"]


def test_clear_resets_registry(tmp_path):
    """clear() should remove all prompts from the registry."""
    make_prompt_file(tmp_path, "scoping", "1.0.0")
    PromptService.load_all(str(tmp_path))

    PromptService.clear()

    with pytest.raises(KeyError):
        PromptService.get("scoping", "1.0.0")


def test_required_fields_validation(tmp_path):
    """A prompt file missing required fields should raise ValueError on load_all."""
    # Missing system_prompt
    incomplete_data = {
        "version": "1.0.0",
        "name": "broken",
        "created_at": "2026-02-27",
        "user_prompt_template": "Hello {user_message}",
        # system_prompt deliberately omitted
    }
    (tmp_path / "v1.0.0-broken.json").write_text(
        json.dumps(incomplete_data), encoding="utf-8"
    )

    with pytest.raises(ValueError, match="missing required fields"):
        PromptService.load_all(str(tmp_path))


def test_load_all_idempotent_same_content(tmp_path):
    """Calling load_all twice with the same files should not raise."""
    make_prompt_file(tmp_path, "scoping", "1.0.0")

    PromptService.load_all(str(tmp_path))
    # Second call with same files — should be a no-op (same hash)
    PromptService.load_all(str(tmp_path))

    result = PromptService.get("scoping", "1.0.0")
    assert result["version"] == "1.0.0"


def test_load_real_prompt_files():
    """Smoke test: load the actual prompt files from config/prompts/."""
    import os
    # Navigate from test file location to config/prompts/
    base = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
    prompts_dir = os.path.join(base, "config", "prompts")

    PromptService.load_all(prompts_dir)

    # Verify all three prompts load
    scoping = PromptService.get("scoping", "latest")
    narrative = PromptService.get("narrative", "latest")
    estimation = PromptService.get("estimation", "latest")

    assert scoping["version"] == "1.0.0"
    assert narrative["version"] == "1.0.0"
    assert estimation["version"] == "1.0.0"

    # Verify version strings
    assert PromptService.get_version_string("scoping", "latest") == "1.0.0"
    assert PromptService.get_version_string("narrative", "latest") == "1.0.0"
    assert PromptService.get_version_string("estimation", "latest") == "1.0.0"
