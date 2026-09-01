"""Mutation tests for the pinned, offline Spec Kit integration."""

from __future__ import annotations

from contextlib import redirect_stderr
import io
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

from scripts_ci import check_speckit_integration


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts_ci" / "check_speckit_integration.py"
METADATA = (
    ".specify/init-options.json",
    ".specify/integration.json",
    ".specify/integrations/codex.manifest.json",
    ".specify/integrations/speckit.manifest.json",
)
MANIFESTS = METADATA[2:]


class SpecKitIntegrationCheckerTest(unittest.TestCase):
    maxDiff = None

    def _materialize(self) -> Path:
        root = Path(tempfile.mkdtemp(prefix="batuta-speckit-"))
        self.addCleanup(shutil.rmtree, root)

        for relative_text in METADATA:
            source = ROOT / relative_text
            target = root / relative_text
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)

        for manifest_text in MANIFESTS:
            manifest = json.loads((ROOT / manifest_text).read_text(encoding="utf-8"))
            for relative_text in manifest["files"]:
                source = ROOT / relative_text
                target = root / relative_text
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(source, target)

        scripts = root / "scripts_ci"
        scripts.mkdir(exist_ok=True)
        if SCRIPT.is_file():
            shutil.copyfile(SCRIPT, scripts / SCRIPT.name)
        return root

    def _run(
        self, root: Path, *arguments: str
    ) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, "scripts_ci/check_speckit_integration.py", *arguments],
            cwd=root,
            text=True,
            capture_output=True,
            check=False,
        )

    @staticmethod
    def _mutate_json(root: Path, relative: str, mutation: object) -> None:
        path = root / relative
        value = json.loads(path.read_text(encoding="utf-8"))
        mutation(value)  # type: ignore[operator]
        path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")

    def _assert_failure(self, root: Path, code: str, exit_code: int = 1) -> None:
        first = self._run(root)
        second = self._run(root)
        self.assertEqual(exit_code, first.returncode, first.stderr)
        self.assertIn(f"[{code}] ", first.stderr)
        self.assertNotIn(str(root), first.stderr)
        self.assertEqual(
            (first.returncode, first.stdout, first.stderr),
            (second.returncode, second.stdout, second.stderr),
        )

    def test_valid_fixture_and_real_repository_are_accepted(self) -> None:
        root = self._materialize()
        fixture = self._run(root)
        fixture_repeat = self._run(root)
        repository = self._run(ROOT)
        repository_repeat = self._run(ROOT)

        self.assertEqual(0, fixture.returncode, fixture.stderr)
        self.assertEqual(0, repository.returncode, repository.stderr)
        expected = "Spec Kit 1.0.2/codex: 22 managed files verified\n"
        self.assertEqual(expected, fixture.stdout)
        self.assertEqual(expected, repository.stdout)
        self.assertEqual("", fixture.stderr)
        self.assertEqual(
            (fixture.returncode, fixture.stdout, fixture.stderr),
            (fixture_repeat.returncode, fixture_repeat.stdout, fixture_repeat.stderr),
        )
        self.assertEqual(
            (repository.returncode, repository.stdout, repository.stderr),
            (
                repository_repeat.returncode,
                repository_repeat.stdout,
                repository_repeat.stderr,
            ),
        )

    def test_version_and_integration_mutations_are_rejected(self) -> None:
        cases = (
            (
                ".specify/init-options.json",
                lambda value: value.__setitem__("speckit_version", "1.0.1"),
            ),
            (
                ".specify/integration.json",
                lambda value: value.__setitem__("version", "1.0.1"),
            ),
            (
                ".specify/integration.json",
                lambda value: value.__setitem__("integration", "claude"),
            ),
            (
                ".specify/integrations/codex.manifest.json",
                lambda value: value.__setitem__("version", "1.0.1"),
            ),
        )
        for relative, mutation in cases:
            with self.subTest(path=relative, mutation=mutation):
                root = self._materialize()
                self._mutate_json(root, relative, mutation)
                self._assert_failure(root, "SPECKIT_VERSION_MISMATCH")

    def test_manifests_are_closed_and_have_exact_file_sets(self) -> None:
        root = self._materialize()
        self._mutate_json(
            root,
            ".specify/integrations/codex.manifest.json",
            lambda value: value.__setitem__("unexpected", True),
        )
        self._assert_failure(root, "SPECKIT_MANIFEST_INVALID")

        root = self._materialize()

        def remove_file(value: dict[str, object]) -> None:
            files = value["files"]
            assert isinstance(files, dict)
            del files[next(iter(files))]

        self._mutate_json(
            root, ".specify/integrations/speckit.manifest.json", remove_file
        )
        self._assert_failure(root, "SPECKIT_MANIFEST_INVALID")

    def test_absolute_parent_and_outside_root_paths_are_rejected(self) -> None:
        unsafe_paths = ("/tmp/escape", "../escape", ".specify/link/escape")
        for unsafe in unsafe_paths:
            with self.subTest(path=unsafe):
                root = self._materialize()
                if unsafe.startswith(".specify/link"):
                    (root / ".specify/link").symlink_to(Path(tempfile.gettempdir()))

                def replace_path(value: dict[str, object]) -> None:
                    files = value["files"]
                    assert isinstance(files, dict)
                    digest = files.pop(next(iter(files)))
                    files[unsafe] = digest

                self._mutate_json(
                    root,
                    ".specify/integrations/codex.manifest.json",
                    replace_path,
                )
                self._assert_failure(root, "SPECKIT_MANAGED_PATH_INVALID")

    def test_missing_managed_file_is_rejected(self) -> None:
        root = self._materialize()
        target = root / ".agents/skills/speckit-analyze/SKILL.md"
        target.unlink()
        self._assert_failure(root, "SPECKIT_MANAGED_FILE_MISSING")

    def test_changed_managed_file_is_rejected(self) -> None:
        root = self._materialize()
        target = root / ".specify/templates/spec-template.md"
        target.write_bytes(target.read_bytes() + b"changed")
        self._assert_failure(root, "SPECKIT_MANAGED_HASH_MISMATCH")

    def test_invalid_arguments_return_two_deterministically(self) -> None:
        root = self._materialize()
        first = self._run(root, "--unexpected")
        second = self._run(root, "--unexpected")
        self.assertEqual(2, first.returncode)
        self.assertEqual((first.stdout, first.stderr), (second.stdout, second.stderr))

    def test_unreadable_root_returns_two(self) -> None:
        original = check_speckit_integration.ROOT
        with tempfile.TemporaryDirectory() as parent:
            missing = Path(parent) / "missing"
            first = io.StringIO()
            second = io.StringIO()
            try:
                check_speckit_integration.ROOT = missing
                with redirect_stderr(first):
                    self.assertEqual(2, check_speckit_integration.main([]))
                with redirect_stderr(second):
                    self.assertEqual(2, check_speckit_integration.main([]))
            finally:
                check_speckit_integration.ROOT = original
            self.assertEqual(first.getvalue(), second.getvalue())
            self.assertNotIn(parent, first.getvalue())


if __name__ == "__main__":
    unittest.main()
