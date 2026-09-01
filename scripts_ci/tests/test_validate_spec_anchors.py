"""Behavior and mutation tests for the spec-anchor registry validator."""

from __future__ import annotations

import copy
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts_ci" / "validate_spec_anchors.py"
FIXTURES = pathlib.Path(__file__).parent / "fixtures" / "spec_anchors"
VALID = FIXTURES / "valid"
INVALID = FIXTURES / "invalid"
IMPACT_INVALID = FIXTURES / "impact_invalid"
IMPACT_PATH = pathlib.Path("specs/001-example/feature-impact.json")
GIT_WARNING = (
    "[GIT_DIFF_OMITTED] .#git: base Git no proporcionada; "
    "correlación de diff omitida\n"
)


def _resolve_pointer(document: object, pointer: str) -> tuple[object, str]:
    parts = [part.replace("~1", "/").replace("~0", "~") for part in pointer.split("/")[1:]]
    parent = document
    for part in parts[:-1]:
        parent = parent[int(part)] if isinstance(parent, list) else parent[part]
    return parent, parts[-1]


def _apply_operation(document: object, operation: dict[str, object]) -> None:
    name = operation["operation"]
    if name == "reverse":
        parent, key = _resolve_pointer(document, str(operation["pointer"]))
        value = parent[int(key)] if isinstance(parent, list) else parent[key]
        value.reverse()
        return

    parent, key = _resolve_pointer(document, str(operation["pointer"]))
    if name == "set" or name == "add":
        value = copy.deepcopy(operation["value"])
        if isinstance(parent, list):
            parent[int(key)] = value
        else:
            parent[key] = value
    elif name == "duplicate":
        value = parent[int(key)] if isinstance(parent, list) else parent[key]
        parent.append(copy.deepcopy(value))
    elif name == "delete":
        if isinstance(parent, list):
            del parent[int(key)]
        else:
            del parent[key]
    else:
        raise AssertionError(f"unknown fixture operation: {name}")


class SpecAnchorValidatorTest(unittest.TestCase):
    maxDiff = None

    def _materialize(
        self,
        mutation: pathlib.Path | None = None,
        *,
        impact_mutation: pathlib.Path | None = None,
    ) -> pathlib.Path:
        temporary = pathlib.Path(tempfile.mkdtemp(prefix="batuta-spec-anchors-"))
        self.addCleanup(shutil.rmtree, temporary)
        shutil.copytree(VALID, temporary, dirs_exist_ok=True)
        if mutation is not None:
            case = json.loads(mutation.read_text(encoding="utf-8"))
            registry_path = temporary / "specs" / "anchors.json"
            registry = json.loads(registry_path.read_text(encoding="utf-8"))
            for operation in case["operations"]:
                _apply_operation(registry, operation)
            registry_path.write_text(
                json.dumps(registry, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
        if impact_mutation is not None:
            case = json.loads(impact_mutation.read_text(encoding="utf-8"))
            impact_path = temporary / IMPACT_PATH
            impact = json.loads(impact_path.read_text(encoding="utf-8"))
            for operation in case["operations"]:
                _apply_operation(impact, operation)
            impact_path.write_text(
                json.dumps(impact, ensure_ascii=False, indent=2) + "\n",
                encoding="utf-8",
            )
        scripts = temporary / "scripts_ci"
        scripts.mkdir(exist_ok=True)
        if not SCRIPT.is_file():
            self.fail("validate_spec_anchors.py must exist before anchor validation can pass")
        shutil.copy2(SCRIPT, scripts / SCRIPT.name)
        return temporary

    def _run(
        self,
        root: pathlib.Path,
        *arguments: str,
        environment: dict[str, str] | None = None,
    ) -> subprocess.CompletedProcess[str]:
        isolated_environment = os.environ.copy() if environment is None else environment.copy()
        if environment is None:
            isolated_environment.pop("BATUTA_SPEC_BASE", None)
        return subprocess.run(
            [sys.executable, "scripts_ci/validate_spec_anchors.py", *arguments],
            cwd=root,
            text=True,
            capture_output=True,
            check=False,
            env=isolated_environment,
        )

    def _git(self, root: pathlib.Path, *arguments: str) -> str:
        result = subprocess.run(
            ["git", *arguments],
            cwd=root,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout.strip()

    def _commit_fixture(self, root: pathlib.Path, message: str) -> str:
        self._git(root, "add", "-A")
        self._git(
            root,
            "-c",
            "user.name=Batuta Tests",
            "-c",
            "user.email=tests@batuta.invalid",
            "commit",
            "-q",
            "-m",
            message,
        )
        return self._git(root, "rev-parse", "HEAD")

    def _initialize_git(self, root: pathlib.Path) -> str:
        self._git(root, "init", "-q")
        return self._commit_fixture(root, "fixture base")

    def test_valid_closed_registry_is_accepted(self) -> None:
        result = self._run(self._materialize())

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "validated 2 capabilities and 2 requirements\n")
        self.assertEqual(result.stderr, GIT_WARNING)

    def test_invalid_fixtures_emit_their_stable_diagnostic(self) -> None:
        cases = sorted(INVALID.glob("*.json"))
        self.assertGreaterEqual(len(cases), 10)
        for path in cases:
            with self.subTest(case=path.stem):
                case = json.loads(path.read_text(encoding="utf-8"))
                result = self._run(self._materialize(path))
                self.assertEqual(result.returncode, case["exit_code"], result.stderr)
                self.assertIn(f"[{case['diagnostic']}] ", result.stderr)
                self.assertNotIn("batuta-spec-anchors-", result.stderr)

    def test_feature_impact_shape_mutations_are_rejected(self) -> None:
        cases = sorted(IMPACT_INVALID.glob("*.json"))
        self.assertGreaterEqual(len(cases), 16)
        for path in cases:
            with self.subTest(case=path.stem):
                case = json.loads(path.read_text(encoding="utf-8"))
                result = self._run(self._materialize(impact_mutation=path))
                self.assertEqual(result.returncode, case["exit_code"], result.stderr)
                self.assertIn(f"[{case['diagnostic']}] ", result.stderr)
                self.assertNotIn("batuta-spec-anchors-", result.stderr)

    def test_feature_impact_schema_declares_a_closed_draft_2020_12_object(self) -> None:
        schema_path = ROOT / "specs" / "schemas" / "feature-impact-v1.schema.json"
        schema = json.loads(schema_path.read_text(encoding="utf-8"))

        self.assertEqual(schema["$schema"], "https://json-schema.org/draft/2020-12/schema")
        self.assertEqual(schema["type"], "object")
        self.assertIs(schema["additionalProperties"], False)
        self.assertEqual(schema["title"], "FeatureImpactV1")

    def test_git_base_modes_are_explicit_and_deterministic(self) -> None:
        root = self._materialize()
        base = self._initialize_git(root)

        valid = self._run(root, "--base", base)
        missing = self._run(root, "--base", "ref-inexistente")
        environment = os.environ.copy()
        environment["BATUTA_SPEC_BASE"] = base
        from_environment = self._run(root, environment=environment)

        self.assertEqual(valid.returncode, 0, valid.stderr)
        self.assertEqual(valid.stderr, "")
        self.assertEqual(from_environment.returncode, 0, from_environment.stderr)
        self.assertEqual(from_environment.stderr, "")
        self.assertEqual(missing.returncode, 2)
        self.assertIn("[GIT_BASE_UNRESOLVABLE] ", missing.stderr)

        again = self._run(root, "--base", "ref-inexistente")
        self.assertEqual(
            (missing.returncode, missing.stdout, missing.stderr),
            (again.returncode, again.stdout, again.stderr),
        )

    def test_product_change_without_active_impact_is_rejected_or_reported(self) -> None:
        root = self._materialize()
        base = self._initialize_git(root)
        (root / IMPACT_PATH).unlink()
        source = root / "crates/example/src/lib.rs"
        source.write_text(source.read_text(encoding="utf-8") + "\npub fn changed() {}\n", encoding="utf-8")
        self._commit_fixture(root, "product without impact")

        enforced = self._run(root, "--base", base)
        reported = self._run(root, "--base", base, "--report")

        self.assertEqual(enforced.returncode, 1, enforced.stderr)
        self.assertIn("[IMPACT_REQUIRED] ", enforced.stderr)
        self.assertEqual(reported.returncode, 0, reported.stderr)
        self.assertIn("[IMPACT_REQUIRED] ", reported.stderr)

    def test_docs_only_cannot_cover_a_normative_change(self) -> None:
        root = self._materialize()
        base = self._initialize_git(root)
        impact_path = root / IMPACT_PATH
        impact = json.loads(impact_path.read_text(encoding="utf-8"))
        impact["change_type"] = "docs_only"
        impact["living_specs_updated"] = False
        impact["rollback"] = {
            "strategy": "not_applicable",
            "procedure": None,
            "success_check": None,
        }
        impact_path.write_text(json.dumps(impact, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        spec = root / "specs/system/product.md"
        spec.write_text(spec.read_text(encoding="utf-8") + "\nCambio normativo.\n", encoding="utf-8")
        self._commit_fixture(root, "misclassified normative change")

        result = self._run(root, "--base", base)

        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn("[IMPACT_CHANGE_TYPE_CONTRACT] ", result.stderr)

    def test_active_impact_covers_a_registered_product_path(self) -> None:
        root = self._materialize()
        base = self._initialize_git(root)
        source = root / "crates/example/src/lib.rs"
        source.write_text(source.read_text(encoding="utf-8") + "\npub fn changed() {}\n", encoding="utf-8")
        impact_path = root / IMPACT_PATH
        impact = json.loads(impact_path.read_text(encoding="utf-8"))
        impact["compatibility"]["notes"] = "El impacto activo cubre el cambio del contrato."
        impact_path.write_text(json.dumps(impact, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        self._commit_fixture(root, "covered product change")

        result = self._run(root, "--base", base)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, "")

    def test_invalid_output_and_exit_code_are_byte_stable(self) -> None:
        mutation = IMPACT_INVALID / "unknown_nested_field.json"
        root = self._materialize(impact_mutation=mutation)

        first = self._run(root)
        second = self._run(root)

        self.assertEqual(
            (first.returncode, first.stdout, first.stderr),
            (second.returncode, second.stdout, second.stderr),
        )


if __name__ == "__main__":
    unittest.main()
