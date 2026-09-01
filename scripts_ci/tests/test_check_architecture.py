"""Fixture tests for Batuta's local-crate dependency boundaries."""

from __future__ import annotations

import json
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts_ci" / "check_architecture.py"
FIXTURES = pathlib.Path(__file__).parent / "fixtures" / "architecture"
VALID = FIXTURES / "valid"
INVALID = FIXTURES / "invalid"


class ArchitectureCheckerTest(unittest.TestCase):
    maxDiff = None

    def _materialize(self, mutation: pathlib.Path | None = None) -> pathlib.Path:
        root = pathlib.Path(tempfile.mkdtemp(prefix="batuta-architecture-"))
        self.addCleanup(shutil.rmtree, root)
        shutil.copytree(VALID, root, dirs_exist_ok=True)
        if mutation is not None:
            case = json.loads(mutation.read_text(encoding="utf-8"))
            for source, target in case["dependencies"]:
                manifest = root / "crates" / source / "Cargo.toml"
                with manifest.open("a", encoding="utf-8") as stream:
                    stream.write(f'{target} = {{ path = "../{target}" }}\n')
        scripts = root / "scripts_ci"
        scripts.mkdir()
        if SCRIPT.is_file():
            shutil.copy2(SCRIPT, scripts / SCRIPT.name)
        return root

    def _run(self, root: pathlib.Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, "scripts_ci/check_architecture.py"],
            cwd=root,
            text=True,
            capture_output=True,
            check=False,
        )

    def test_valid_fixture_and_real_repository_are_accepted(self) -> None:
        fixture = self._run(self._materialize())
        repository = self._run(ROOT)

        self.assertEqual(fixture.returncode, 0, fixture.stderr)
        self.assertEqual(repository.returncode, 0, repository.stderr)
        self.assertIn("validated 3 local crates", fixture.stdout)
        self.assertIn("validated 10 local crates", repository.stdout)

    def test_architecture_mutations_emit_stable_diagnostics(self) -> None:
        cases = sorted(INVALID.glob("*.json"))
        self.assertEqual(len(cases), 3)
        for path in cases:
            with self.subTest(case=path.stem):
                case = json.loads(path.read_text(encoding="utf-8"))
                result = self._run(self._materialize(path))
                self.assertEqual(result.returncode, case["exit_code"], result.stderr)
                self.assertIn(f"[{case['diagnostic']}] ", result.stderr)
                self.assertNotIn("batuta-architecture-", result.stderr)

    def test_cycle_result_is_byte_stable(self) -> None:
        root = self._materialize(INVALID / "cycle.json")

        first = self._run(root)
        second = self._run(root)

        self.assertEqual(
            (first.returncode, first.stdout, first.stderr),
            (second.returncode, second.stdout, second.stderr),
        )


if __name__ == "__main__":
    unittest.main()
