"""
Prompt registry for versioned, immutable prompt management.

Prompts are stored as JSON files in config/prompts/ and loaded once at app startup.
Each prompt has a name (e.g., "scoping", "narrative", "estimation") and a semver
version string. Versions are immutable — any change to content requires a new version.

Usage:
    PromptService.load_all("config/prompts/")  # called in app lifespan
    prompt = PromptService.get("narrative", "1.0.0")
    prompt = PromptService.get("narrative", "latest")  # resolves to highest semver
"""

import hashlib
import json
import logging
from pathlib import Path

logger = logging.getLogger(__name__)

# Required fields for a valid prompt file
_REQUIRED_FIELDS = {
    "version",
    "name",
    "created_at",
    "system_prompt",
    "user_prompt_template",
}


def _semver_key(version_str: str) -> tuple:
    """Parse a semver string into a sortable tuple of ints."""
    return tuple(int(x) for x in version_str.split("."))


class PromptService:
    """Registry for versioned, immutable prompt files.

    All methods are classmethods — PromptService is used as a singleton registry.
    Call ``load_all()`` once at app startup. Use ``get()`` to retrieve prompts at runtime.  # noqa: E501
    """

    # Class-level state: keyed by "{name}:{version}"
    _registry: dict[str, dict] = {}
    _content_hashes: dict[str, str] = {}

    @classmethod
    def load_all(cls, prompts_dir: str) -> None:
        """Load all prompt JSON files from prompts_dir into the registry.

        Args:
            prompts_dir: Filesystem path to the directory containing prompt JSON files.

        Raises:
            ValueError: If a prompt file is missing required fields or if an already-loaded  # noqa: E501
                        version's content has changed (immutability violation).
        """
        path = Path(prompts_dir)
        loaded_count = 0

        for json_file in path.glob("*.json"):
            content = json_file.read_text(encoding="utf-8")
            content_hash = hashlib.sha256(content.encode("utf-8")).hexdigest()

            try:
                data = json.loads(content)
            except json.JSONDecodeError as e:
                raise ValueError(
                    f"Invalid JSON in prompt file {json_file.name}: {e}"
                ) from e

            # Validate required fields
            missing = _REQUIRED_FIELDS - set(data.keys())
            if missing:
                raise ValueError(
                    f"Prompt file {json_file.name} is missing required fields: {missing}"  # noqa: E501
                )

            key = f"{data['name']}:{data['version']}"

            # Immutability check: if key exists and hash differs, raise
            if key in cls._content_hashes:
                if cls._content_hashes[key] != content_hash:
                    raise ValueError(
                        f"Immutability violation: {key} content changed. "
                        "Create a new version instead of modifying an existing one."
                    )
                # Same content, already loaded — skip silently
                continue

            cls._registry[key] = data
            cls._content_hashes[key] = content_hash
            loaded_count += 1
            logger.debug("Loaded prompt: %s", key)

        logger.info(
            "Prompt registry loaded: %d prompt(s) from %s", loaded_count, prompts_dir
        )

    @classmethod
    def get(cls, name: str, version: str = "latest") -> dict:
        """Retrieve a prompt by name and version.

        Args:
            name:    Prompt name (e.g., "scoping", "narrative", "estimation").
            version: Exact semver string (e.g., "1.0.0") or "latest" (resolves to highest).  # noqa: E501

        Returns:
            The prompt dict (contains version, name, system_prompt, user_prompt_template, etc.)  # noqa: E501

        Raises:
            KeyError: If no matching prompt is found.
        """
        if version == "latest":
            candidates = [
                (cls._semver_key(v), cls._registry[f"{name}:{v}"])
                for k, _ in cls._registry.items()
                if ":" in k and k.split(":", 1)[0] == name
                for v in [k.split(":", 1)[1]]
            ]
            if not candidates:
                raise KeyError(
                    f"Prompt not found: {name}:latest (no versions registered)"
                )
            candidates.sort(key=lambda x: x[0], reverse=True)
            return candidates[0][1]
        else:
            key = f"{name}:{version}"
            if key not in cls._registry:
                raise KeyError(f"Prompt not found: {key}")
            return cls._registry[key]

    @classmethod
    def get_version_string(cls, name: str, version: str = "latest") -> str:
        """Return the resolved version string for a prompt.

        Convenience helper for populating the ``prompt_version`` field on
        EstimationSession records.

        Args:
            name:    Prompt name.
            version: Exact version or "latest".

        Returns:
            The version string (e.g., "1.0.0").
        """
        return cls.get(name, version)["version"]

    @classmethod
    def list_versions(cls, name: str) -> list[str]:
        """Return all registered versions for a prompt name, sorted ascending by semver.

        Args:
            name: Prompt name.

        Returns:
            Sorted list of version strings (e.g., ["1.0.0", "1.1.0"]).
        """
        versions = [
            k.split(":", 1)[1] for k in cls._registry if k.split(":", 1)[0] == name
        ]
        return sorted(versions, key=_semver_key)

    @classmethod
    def _semver_key(cls, version_str: str) -> tuple:
        """Parse a semver string into a sortable tuple of ints."""
        return _semver_key(version_str)

    @classmethod
    def clear(cls) -> None:
        """Reset the registry (for test isolation)."""
        cls._registry = {}
        cls._content_hashes = {}
