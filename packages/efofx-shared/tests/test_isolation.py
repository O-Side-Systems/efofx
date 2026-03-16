"""
Import isolation test for efofx-shared.

Verifies that efofx-shared can be installed and imported in a fresh virtual
environment that has NO fastapi, motor, or uvicorn installed.

This test guards the dependency boundary: efofx-shared must remain a pure
framework-agnostic package so a future IT/dev vertical can consume it without
pulling in any estimation-service application-server dependencies.
"""

import subprocess
import sys
import venv
from pathlib import Path


def _run(cmd: list[str], cwd: str | None = None) -> subprocess.CompletedProcess:
    return subprocess.run(
        cmd,
        capture_output=True,
        text=True,
        cwd=cwd,
    )


def test_no_app_imports(tmp_path: Path) -> None:
    """efofx-shared installs cleanly without app-server dependencies."""
    venv_dir = tmp_path / "isolated_venv"

    # 1. Create a fresh virtual environment.
    venv.create(str(venv_dir), with_pip=True, clear=True)

    python = str(venv_dir / "bin" / "python")
    pip = str(venv_dir / "bin" / "pip")

    # 2. Locate the efofx-shared package root (two levels up from this file).
    package_root = Path(__file__).parent.parent.resolve()

    # 3. Install efofx-shared using uv pip (faster, avoids private-index prompts).
    #    Fall back to pip if uv is not available.
    try:
        import shutil

        uv_bin = shutil.which("uv")
        if uv_bin:
            result = _run(
                [uv_bin, "pip", "install", "--python", python, "-e", str(package_root)]
            )
        else:
            result = _run([pip, "install", "-e", str(package_root)])

        assert result.returncode == 0, (
            f"efofx-shared install failed:\nSTDOUT: {result.stdout}\nSTDERR: {result.stderr}"
        )
    except Exception as exc:
        raise AssertionError(f"Install step raised unexpectedly: {exc}") from exc

    # 4. Assert efofx_shared imports successfully.
    import_result = _run(
        [python, "-c", "import efofx_shared; print(efofx_shared.__version__)"]
    )
    assert import_result.returncode == 0, (
        f"'import efofx_shared' failed in isolated venv:\n"
        f"STDOUT: {import_result.stdout}\nSTDERR: {import_result.stderr}"
    )

    # 5. Assert prohibited app-server packages are NOT importable.
    prohibited = ["fastapi", "motor", "uvicorn"]
    for pkg in prohibited:
        leak_result = _run([python, "-c", f"import {pkg}"])
        assert leak_result.returncode != 0, (
            f"'{pkg}' should NOT be importable in isolated efofx-shared venv "
            f"but import succeeded — dependency leak detected!"
        )
