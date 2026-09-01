#!/usr/bin/env python3
"""Verify Batuta's pinned Spec Kit integration without running Spec Kit."""

from __future__ import annotations

from dataclasses import dataclass
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import sys
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
VERSION = "1.0.2"
INTEGRATION = "codex"
INIT_OPTIONS = ".specify/init-options.json"
INTEGRATION_STATE = ".specify/integration.json"
MANIFEST_FIELDS = {"integration", "version", "installed_at", "files"}
SHA256 = re.compile(r"[0-9a-f]{64}\Z")

CODEX_FILES = frozenset(
    {
        ".agents/skills/speckit-analyze/SKILL.md",
        ".agents/skills/speckit-clarify/SKILL.md",
        ".agents/skills/speckit-constitution/SKILL.md",
        ".agents/skills/speckit-implement/SKILL.md",
        ".agents/skills/speckit-converge/SKILL.md",
        ".agents/skills/speckit-plan/SKILL.md",
        ".agents/skills/speckit-checklist/SKILL.md",
        ".agents/skills/speckit-specify/SKILL.md",
        ".agents/skills/speckit-tasks/SKILL.md",
        ".agents/skills/speckit-taskstoissues/SKILL.md",
    }
)
SPECKIT_FILES = frozenset(
    {
        ".specify/scripts/bash/create-new-feature.sh",
        ".specify/scripts/bash/resolve-template.sh",
        ".specify/scripts/bash/setup-plan.sh",
        ".specify/scripts/bash/setup-tasks.sh",
        ".specify/scripts/bash/check-prerequisites.sh",
        ".specify/scripts/bash/common.sh",
        ".specify/templates/checklist-template.md",
        ".specify/templates/constitution-template.md",
        ".specify/templates/tasks-template.md",
        ".specify/templates/spec-template.md",
        ".specify/templates/plan-template.md",
        ".specify/.gitignore",
    }
)
MANIFESTS = (
    (
        ".specify/integrations/codex.manifest.json",
        "codex",
        CODEX_FILES,
    ),
    (
        ".specify/integrations/speckit.manifest.json",
        "speckit",
        SPECKIT_FILES,
    ),
)


@dataclass(frozen=True, order=True)
class Diagnostic:
    code: str
    path: str
    anchor: str
    detail: str

    def render(self) -> str:
        return f"[{self.code}] {self.path}#{self.anchor}: {self.detail}"


class RootUnreadable(Exception):
    """The repository root itself cannot be inspected."""


class Validation:
    def __init__(self, root: Path) -> None:
        try:
            self.root = root.resolve(strict=True)
        except OSError as error:
            raise RootUnreadable("raíz no resoluble") from error
        if not self.root.is_dir():
            raise RootUnreadable("la raíz no es un directorio")
        try:
            next(self.root.iterdir(), None)
        except OSError as error:
            raise RootUnreadable("raíz no legible") from error
        self.diagnostics: list[Diagnostic] = []
        self.managed: dict[str, str] = {}

    def add(self, code: str, path: str, anchor: str, detail: str) -> None:
        self.diagnostics.append(Diagnostic(code, path, anchor, detail))

    def read_json(self, relative: str) -> dict[str, Any] | None:
        try:
            value = json.loads((self.root / relative).read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError):
            self.add(
                "SPECKIT_MANIFEST_INVALID",
                relative,
                "json",
                "JSON ausente, ilegible o inválido",
            )
            return None
        if not isinstance(value, dict):
            self.add(
                "SPECKIT_MANIFEST_INVALID",
                relative,
                "json",
                "se esperaba un objeto JSON",
            )
            return None
        return value

    def check_configuration(self) -> None:
        options = self.read_json(INIT_OPTIONS)
        if options is not None:
            expected = {
                "speckit_version": VERSION,
                "integration": INTEGRATION,
                "ai": INTEGRATION,
            }
            for field, wanted in expected.items():
                if options.get(field) != wanted:
                    self.add(
                        "SPECKIT_VERSION_MISMATCH",
                        INIT_OPTIONS,
                        field,
                        f"se esperaba {wanted!r}",
                    )

        state = self.read_json(INTEGRATION_STATE)
        if state is None:
            return
        expected_state = {
            "version": VERSION,
            "integration": INTEGRATION,
            "default_integration": INTEGRATION,
            "installed_integrations": [INTEGRATION],
            "integration_settings": {
                INTEGRATION: {"script": "sh", "invoke_separator": "-"}
            },
        }
        for field, wanted in expected_state.items():
            if state.get(field) != wanted:
                self.add(
                    "SPECKIT_VERSION_MISMATCH",
                    INTEGRATION_STATE,
                    field,
                    f"se esperaba {wanted!r}",
                )
        if state.get("integration_state_schema") != 1:
            self.add(
                "SPECKIT_MANIFEST_INVALID",
                INTEGRATION_STATE,
                "integration_state_schema",
                "se esperaba 1",
            )

    def path_is_safe(self, relative: str) -> bool:
        pure = PurePosixPath(relative)
        if (
            not relative
            or "\\" in relative
            or pure.is_absolute()
            or any(part in {"", ".", ".."} for part in relative.split("/"))
        ):
            return False
        try:
            (self.root / relative).resolve(strict=False).relative_to(self.root)
        except (OSError, ValueError):
            return False
        return True

    def check_manifest(
        self,
        relative: str,
        integration: str,
        expected_files: frozenset[str],
    ) -> None:
        manifest = self.read_json(relative)
        if manifest is None:
            return
        fields = set(manifest)
        if fields != MANIFEST_FIELDS:
            missing = sorted(MANIFEST_FIELDS - fields)
            extra = sorted(fields - MANIFEST_FIELDS)
            self.add(
                "SPECKIT_MANIFEST_INVALID",
                relative,
                "fields",
                f"campos ausentes={missing!r}; desconocidos={extra!r}",
            )
        if manifest.get("integration") != integration:
            self.add(
                "SPECKIT_VERSION_MISMATCH",
                relative,
                "integration",
                f"se esperaba {integration!r}",
            )
        if manifest.get("version") != VERSION:
            self.add(
                "SPECKIT_VERSION_MISMATCH",
                relative,
                "version",
                f"se esperaba {VERSION!r}",
            )
        if not isinstance(manifest.get("installed_at"), str) or not manifest.get(
            "installed_at"
        ):
            self.add(
                "SPECKIT_MANIFEST_INVALID",
                relative,
                "installed_at",
                "se esperaba un string no vacío",
            )

        files = manifest.get("files")
        if not isinstance(files, dict):
            self.add(
                "SPECKIT_MANIFEST_INVALID",
                relative,
                "files",
                "se esperaba un objeto ruta-hash",
            )
            return
        actual_files = set(files)
        if actual_files != expected_files:
            self.add(
                "SPECKIT_MANIFEST_INVALID",
                relative,
                "files",
                "el conjunto de ficheros administrados no es el esperado",
            )
        for path, digest in sorted(files.items()):
            if not isinstance(path, str) or not self.path_is_safe(path):
                rendered = path if isinstance(path, str) and path else "<non-string>"
                self.add(
                    "SPECKIT_MANAGED_PATH_INVALID",
                    relative,
                    rendered,
                    "la ruta debe ser relativa, POSIX y permanecer dentro de la raíz",
                )
                continue
            if not isinstance(digest, str) or SHA256.fullmatch(digest) is None:
                self.add(
                    "SPECKIT_MANIFEST_INVALID",
                    relative,
                    path,
                    "SHA-256 debe contener 64 caracteres hexadecimales minúsculos",
                )
                continue
            self.managed[path] = digest

    def check_files(self) -> None:
        for relative, expected_digest in sorted(self.managed.items()):
            target = self.root / relative
            try:
                payload = target.read_bytes()
            except (OSError, ValueError):
                self.add(
                    "SPECKIT_MANAGED_FILE_MISSING",
                    relative,
                    "file",
                    "fichero administrado ausente o ilegible",
                )
                continue
            actual_digest = hashlib.sha256(payload).hexdigest()
            if actual_digest != expected_digest:
                self.add(
                    "SPECKIT_MANAGED_HASH_MISMATCH",
                    relative,
                    "sha256",
                    "el contenido no coincide con el manifest",
                )

    def run(self) -> list[Diagnostic]:
        self.check_configuration()
        for relative, integration, expected_files in MANIFESTS:
            self.check_manifest(relative, integration, expected_files)
        self.check_files()
        return sorted(set(self.diagnostics))


def validate_repository(root: Path) -> list[Diagnostic]:
    """Return every deterministic integration diagnostic for ``root``."""

    return Validation(root).run()


def main(argv: list[str] | None = None) -> int:
    arguments = sys.argv[1:] if argv is None else argv
    if arguments:
        print(
            "[SPECKIT_MANIFEST_INVALID] "
            "scripts_ci/check_speckit_integration.py#arguments: "
            "no se admiten argumentos",
            file=sys.stderr,
        )
        return 2
    try:
        diagnostics = validate_repository(ROOT)
    except RootUnreadable as error:
        print(
            "[SPECKIT_MANIFEST_INVALID] .#root: " + str(error),
            file=sys.stderr,
        )
        return 2
    for diagnostic in diagnostics:
        print(diagnostic.render(), file=sys.stderr)
    if diagnostics:
        return 1
    print("Spec Kit 1.0.2/codex: 22 managed files verified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
