"""Mutation tests for Rust module-size limits and registered exceptions."""

from __future__ import annotations

import json
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts_ci" / "check_modularity.py"


def _exception(
    *,
    path: str = "crates/sample/src/lib.rs",
    kind: str = "production",
    characterization: str = "crates/sample/tests/behavior.rs",
) -> dict[str, object]:
    return {
        "path": path,
        "kind": kind,
        "justification": "Deuda heredada cubierta por caracterización.",
        "extraction": "Extraer la responsabilidad de validación a un módulo cohesivo.",
        "characterization_tests": [characterization],
    }


class ModularityCheckerTest(unittest.TestCase):
    maxDiff = None

    def _materialize(
        self,
        *,
        production_lines: int = 20,
        test_lines: int = 20,
        exceptions: list[dict[str, object]] | None = None,
    ) -> pathlib.Path:
        root = pathlib.Path(tempfile.mkdtemp(prefix="batuta-modularity-"))
        self.addCleanup(shutil.rmtree, root)
        production = root / "crates/sample/src/lib.rs"
        test = root / "crates/sample/tests/behavior.rs"
        production.parent.mkdir(parents=True)
        test.parent.mkdir(parents=True)
        production.write_text(
            "".join(f"// production line {index}\n" for index in range(production_lines)),
            encoding="utf-8",
        )
        test.write_text(
            "".join(f"// test line {index}\n" for index in range(test_lines)),
            encoding="utf-8",
        )
        scripts = root / "scripts_ci"
        scripts.mkdir()
        (scripts / "modularity_exceptions.json").write_text(
            json.dumps(
                {"schema_version": 1, "exceptions": exceptions or []},
                ensure_ascii=False,
                indent=2,
            )
            + "\n",
            encoding="utf-8",
        )
        if SCRIPT.is_file():
            shutil.copy2(SCRIPT, scripts / SCRIPT.name)
        return root

    def _run(self, root: pathlib.Path) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, "scripts_ci/check_modularity.py"],
            cwd=root,
            text=True,
            capture_output=True,
            check=False,
        )

    def test_real_repository_respects_limits_and_exact_exceptions(self) -> None:
        result = self._run(ROOT)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("checked ", result.stdout)
        self.assertIn("[MODULARITY_EXCEPTION_ACTIVE] ", result.stderr)

    def test_warning_and_active_exception_do_not_fail(self) -> None:
        warning = self._run(self._materialize(production_lines=400))
        exception = self._run(
            self._materialize(production_lines=500, exceptions=[_exception()])
        )

        self.assertEqual(warning.returncode, 0, warning.stderr)
        self.assertIn("[MODULARITY_REVIEW_WARNING] ", warning.stderr)
        self.assertEqual(exception.returncode, 0, exception.stderr)
        self.assertIn("[MODULARITY_EXCEPTION_ACTIVE] ", exception.stderr)

    def test_threshold_duplicate_stale_path_and_test_mutations_are_rejected(self) -> None:
        cases = [
            (
                "production threshold",
                self._materialize(production_lines=500),
                "MODULARITY_LIMIT_EXCEEDED",
            ),
            (
                "test threshold",
                self._materialize(test_lines=700),
                "MODULARITY_LIMIT_EXCEEDED",
            ),
            (
                "duplicate exception",
                self._materialize(
                    production_lines=500,
                    exceptions=[_exception(), _exception()],
                ),
                "MODULARITY_EXCEPTION_DUPLICATE",
            ),
            (
                "stale exception",
                self._materialize(production_lines=499, exceptions=[_exception()]),
                "MODULARITY_EXCEPTION_STALE",
            ),
            (
                "missing exception path",
                self._materialize(
                    exceptions=[_exception(path="crates/sample/src/missing.rs")],
                ),
                "MODULARITY_CHARACTERIZATION_MISSING",
            ),
            (
                "missing characterization",
                self._materialize(
                    production_lines=500,
                    exceptions=[_exception(characterization="crates/sample/tests/missing.rs")],
                ),
                "MODULARITY_CHARACTERIZATION_MISSING",
            ),
        ]
        for name, root, diagnostic in cases:
            with self.subTest(case=name):
                result = self._run(root)
                self.assertEqual(result.returncode, 1, result.stderr)
                self.assertIn(f"[{diagnostic}] ", result.stderr)
                self.assertNotIn("batuta-modularity-", result.stderr)

    def test_invalid_result_is_byte_stable(self) -> None:
        root = self._materialize(production_lines=500)

        first = self._run(root)
        second = self._run(root)

        self.assertEqual(
            (first.returncode, first.stdout, first.stderr),
            (second.returncode, second.stdout, second.stderr),
        )


if __name__ == "__main__":
    unittest.main()
