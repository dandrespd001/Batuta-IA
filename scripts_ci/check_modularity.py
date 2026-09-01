#!/usr/bin/env python3
"""Check Rust module-size limits and Batuta's closed exception registry."""

from __future__ import annotations

from collections import Counter
from dataclasses import dataclass
import json
import pathlib
import sys


ROOT = pathlib.Path(__file__).resolve().parents[1]
EXCEPTIONS_PATH = pathlib.PurePosixPath("scripts_ci/modularity_exceptions.json")
ROOT_KEYS = {"schema_version", "exceptions"}
EXCEPTION_KEYS = {
    "path",
    "kind",
    "justification",
    "extraction",
    "characterization_tests",
}
KINDS = {"production", "test"}
WARNING_LINES = 400
FAILURE_LINES = {"production": 500, "test": 700}


@dataclass(frozen=True, order=True)
class Diagnostic:
    code: str
    path: str
    anchor: str
    detail: str

    def render(self) -> str:
        return f"[{self.code}] {self.path}#{self.anchor}: {self.detail}"


class Validation:
    def __init__(self, root: pathlib.Path) -> None:
        self.root = root.resolve()
        self.diagnostics: list[Diagnostic] = []
        self.warnings: list[Diagnostic] = []
        self.modules: dict[str, tuple[str, int]] = {}
        self.exception_count = 0

    def add(self, code: str, path: str, anchor: str, detail: str) -> None:
        self.diagnostics.append(Diagnostic(code, path, anchor, detail))

    def warn(self, code: str, path: str, anchor: str, detail: str) -> None:
        self.warnings.append(Diagnostic(code, path, anchor, detail))

    def closed_object(self, value: object, expected: set[str], path: str, anchor: str) -> bool:
        if not isinstance(value, dict):
            self.add("SCHEMA_TYPE_MISMATCH", path, anchor, "se esperaba un objeto")
            return False
        for field in sorted(set(value) - expected):
            self.add("SCHEMA_UNKNOWN_FIELD", path, anchor, f"campo desconocido: {field}")
        for field in sorted(expected - set(value)):
            self.add("SCHEMA_REQUIRED_FIELD", path, anchor, f"falta el campo: {field}")
        return True

    @staticmethod
    def kind_for(relative: pathlib.PurePosixPath) -> str:
        return "test" if "tests" in relative.parts else "production"

    def safe_file(self, value: object) -> pathlib.Path | None:
        if not isinstance(value, str) or not value:
            return None
        relative = pathlib.PurePosixPath(value)
        if (
            relative.is_absolute()
            or "\\" in value
            or value != relative.as_posix()
            or any(part in {"", ".", ".."} for part in relative.parts)
        ):
            return None
        candidate = self.root.joinpath(*relative.parts)
        try:
            resolved = candidate.resolve(strict=True)
            resolved.relative_to(self.root)
        except (OSError, ValueError):
            return None
        return candidate if candidate.is_file() else None

    def discover_modules(self) -> None:
        crates = self.root / "crates"
        if not crates.is_dir():
            self.add("MODULARITY_CHARACTERIZATION_MISSING", "crates", "root", "no existe el árbol de crates")
            return
        for path in sorted(crates.glob("**/*.rs")):
            if not path.is_file():
                continue
            try:
                resolved = path.resolve(strict=True)
                resolved.relative_to(self.root)
                lines = len(path.read_text(encoding="utf-8").splitlines())
            except (OSError, UnicodeError, ValueError):
                relative = path.relative_to(self.root).as_posix()
                self.add("MODULARITY_CHARACTERIZATION_MISSING", relative, "module", "no se puede leer")
                continue
            relative = path.relative_to(self.root).as_posix()
            self.modules[relative] = (self.kind_for(pathlib.PurePosixPath(relative)), lines)

    def validate_exception(
        self,
        value: object,
        duplicate_paths: set[str],
    ) -> str | None:
        source = EXCEPTIONS_PATH.as_posix()
        if not self.closed_object(value, EXCEPTION_KEYS, source, "exception"):
            return None
        assert isinstance(value, dict)
        path_value = value.get("path")
        anchor = path_value if isinstance(path_value, str) else "exception"
        if path_value in duplicate_paths:
            self.add("MODULARITY_EXCEPTION_DUPLICATE", source, anchor, "ruta de excepción duplicada")

        path = self.safe_file(path_value)
        if path is None or not isinstance(path_value, str) or path_value not in self.modules:
            self.add(
                "MODULARITY_CHARACTERIZATION_MISSING",
                source,
                anchor,
                f"ruta de excepción no resoluble: {path_value}",
            )
            return None

        kind = value.get("kind")
        if not isinstance(kind, str):
            self.add("SCHEMA_TYPE_MISMATCH", source, anchor, "kind debe ser texto")
            return None
        if kind not in KINDS:
            self.add("SCHEMA_UNKNOWN_ENUM", source, anchor, f"kind desconocido: {kind}")
            return None
        actual_kind, lines = self.modules[path_value]
        if kind != actual_kind:
            self.add(
                "SCHEMA_CONDITIONAL_INVALID",
                source,
                anchor,
                f"kind {kind}; derivado {actual_kind}",
            )
            return None

        for field in ("justification", "extraction"):
            text = value.get(field)
            if not isinstance(text, str) or not text.strip():
                self.add("SCHEMA_TYPE_MISMATCH", source, anchor, f"{field} debe ser texto no vacío")

        tests = value.get("characterization_tests")
        tests_valid = True
        if not isinstance(tests, list) or not tests or not all(isinstance(item, str) for item in tests):
            self.add(
                "MODULARITY_CHARACTERIZATION_MISSING",
                source,
                anchor,
                "characterization_tests debe ser una lista no vacía de rutas",
            )
            tests_valid = False
        else:
            if tests != sorted(tests) or len(tests) != len(set(tests)):
                self.add("SCHEMA_VALUE_INVALID", source, anchor, "characterization_tests debe estar ordenado y sin duplicados")
                tests_valid = False
            for test in tests:
                pure = pathlib.PurePosixPath(test)
                if self.safe_file(test) is None or "tests" not in pure.parts or pure.suffix != ".rs":
                    self.add(
                        "MODULARITY_CHARACTERIZATION_MISSING",
                        source,
                        anchor,
                        f"prueba de caracterización no resoluble: {test}",
                    )
                    tests_valid = False

        threshold = FAILURE_LINES[kind]
        if lines < threshold:
            self.add(
                "MODULARITY_EXCEPTION_STALE",
                source,
                anchor,
                f"{lines} líneas; la excepción sólo se admite desde {threshold}",
            )
            return None
        self.warn(
            "MODULARITY_EXCEPTION_ACTIVE",
            path_value,
            str(lines),
            f"deuda registrada de {kind}; límite {threshold}",
        )
        return path_value if tests_valid else None

    def run(self) -> None:
        self.discover_modules()
        source = self.root / EXCEPTIONS_PATH
        if not source.is_file():
            self.add(
                "MODULARITY_CHARACTERIZATION_MISSING",
                EXCEPTIONS_PATH.as_posix(),
                "registry",
                "falta el registro de excepciones",
            )
            return
        try:
            value = json.loads(source.read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError) as error:
            self.add("SCHEMA_VALUE_INVALID", EXCEPTIONS_PATH.as_posix(), "registry", f"JSON inválido: {error}")
            return
        if not self.closed_object(value, ROOT_KEYS, EXCEPTIONS_PATH.as_posix(), "registry"):
            return
        assert isinstance(value, dict)
        if type(value.get("schema_version")) is not int or value.get("schema_version") != 1:
            self.add("SCHEMA_VALUE_INVALID", EXCEPTIONS_PATH.as_posix(), "registry", "schema_version debe ser 1")
        exceptions = value.get("exceptions")
        if not isinstance(exceptions, list):
            self.add("SCHEMA_TYPE_MISMATCH", EXCEPTIONS_PATH.as_posix(), "registry", "exceptions debe ser una lista")
            return
        self.exception_count = len(exceptions)
        paths = [item.get("path") for item in exceptions if isinstance(item, dict) and isinstance(item.get("path"), str)]
        duplicates = {path for path, count in Counter(paths).items() if count > 1}
        valid_exceptions = {
            path
            for path in (self.validate_exception(item, duplicates) for item in exceptions)
            if path is not None
        }

        for path, (kind, lines) in sorted(self.modules.items()):
            threshold = FAILURE_LINES[kind]
            if lines >= threshold and path not in valid_exceptions:
                self.add(
                    "MODULARITY_LIMIT_EXCEEDED",
                    path,
                    str(lines),
                    f"módulo {kind} sin excepción válida; límite {threshold}",
                )
            elif WARNING_LINES <= lines < threshold:
                self.warn(
                    "MODULARITY_REVIEW_WARNING",
                    path,
                    str(lines),
                    f"módulo {kind} en zona de revisión; límite {threshold}",
                )


def main(argv: list[str] | None = None) -> int:
    arguments = sys.argv[1:] if argv is None else argv
    if arguments:
        print(
            Diagnostic(
                "USAGE_INVALID",
                "scripts_ci/check_modularity.py",
                "arguments",
                "este checker no admite argumentos",
            ).render(),
            file=sys.stderr,
        )
        return 2
    if not ROOT.is_dir():
        print(
            Diagnostic("MODULARITY_ROOT_UNREADABLE", ".", "root", "no se puede leer la raíz").render(),
            file=sys.stderr,
        )
        return 2
    validation = Validation(ROOT)
    validation.run()
    diagnostics = sorted(set(validation.diagnostics))
    for diagnostic in sorted(set(diagnostics + validation.warnings)):
        print(diagnostic.render(), file=sys.stderr)
    if diagnostics:
        return 1
    print(
        f"checked {len(validation.modules)} Rust modules and "
        f"{validation.exception_count} modularity exceptions"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
