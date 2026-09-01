#!/usr/bin/env python3
"""Validate Batuta's living-spec anchor registry without third-party packages."""

from __future__ import annotations

from collections import Counter
from dataclasses import dataclass
import json
import os
import pathlib
import re
import subprocess
import sys


ROOT = pathlib.Path(__file__).resolve().parents[1]
REGISTRY_PATH = pathlib.PurePosixPath("specs/anchors.json")
ROADMAP_PATH = pathlib.PurePosixPath("ROADMAP.md")
CLASSIFICATION_PATH = pathlib.PurePosixPath("docs/DOCUMENT_CLASSIFICATION.md")
IMPACT_SCHEMA_PATH = pathlib.PurePosixPath("specs/schemas/feature-impact-v1.schema.json")
SYSTEM_SPECS = (
    "specs/system/execution.md",
    "specs/system/manifests.md",
    "specs/system/product.md",
    "specs/system/quality-research.md",
    "specs/system/rollout.md",
    "specs/system/state-policy-routing.md",
    "specs/system/surfaces.md",
)
BASELINE = "7de68af2c9a36ba3dcc65971e4bba83231fb3855"
GENERATED_FROM = "specs/system/product.md"
STATUSES = {"implemented", "partial", "external", "deprecated"}
KINDS = {"test", "gate", "manual_protocol"}
CAPABILITY_ID = re.compile(r"^CAP-[A-Z][A-Z0-9-]*$")
REQUIREMENT_ID = re.compile(r"^REQ-[A-Z][A-Z0-9-]*-[0-9]{3}$")
ROADMAP_ID = re.compile(r"^RM-[0-9]{3}$")
FEATURE_ID = re.compile(r"^[0-9]{3}-[a-z0-9]+(?:-[a-z0-9]+)*$")
CAPABILITY_HEADING = re.compile(r"^##\s+(CAP-[A-Z][A-Z0-9-]*)\b")
REQUIREMENT_HEADING = re.compile(r"^###\s+(REQ-[A-Z][A-Z0-9-]*-[0-9]{3})\b")
ROADMAP_HEADING = re.compile(r"^##\s+(RM-[0-9]{3})\b")

ROOT_KEYS = {"schema_version", "baseline_commit", "generated_from", "capabilities"}
CAPABILITY_KEYS = {
    "id",
    "title",
    "owner_spec",
    "code_paths",
    "status",
    "requirements",
    "evidence",
    "roadmap_id",
    "protocol",
}
REQUIREMENT_KEYS = {"id", "statement", "status", "verifications"}
VERIFICATION_KEYS = {"kind", "path", "selector"}
IMPACT_KEYS = {
    "schema_version",
    "feature_id",
    "change_type",
    "capabilities",
    "requirements",
    "compatibility",
    "migration",
    "rollback",
    "living_specs_updated",
    "characterization",
}
COMPATIBILITY_KEYS = {"public_contract", "persisted_data", "notes"}
MIGRATION_KEYS = {"required", "plan", "backup", "retry"}
ROLLBACK_KEYS = {"strategy", "procedure", "success_check"}
CHANGE_TYPES = {"behavior", "contract", "internal_refactor", "docs_only"}
COMPATIBILITY_VERDICTS = {"compatible", "incompatible", "not_applicable"}
ROLLBACK_STRATEGIES = {"revert", "restore_backup", "forward_fix", "not_applicable"}


@dataclass(frozen=True, order=True)
class Diagnostic:
    code: str
    path: str
    anchor: str
    detail: str

    def render(self) -> str:
        return f"[{self.code}] {self.path}#{self.anchor}: {self.detail}"


@dataclass(frozen=True)
class Impact:
    path: str
    feature_id: str
    change_type: str
    capabilities: tuple[str, ...]
    requirements: tuple[str, ...]


class Validation:
    def __init__(self, root: pathlib.Path) -> None:
        self.root = root.resolve()
        self.diagnostics: list[Diagnostic] = []
        self.warnings: list[Diagnostic] = []
        self.capability_count = 0
        self.requirement_count = 0
        self.capability_paths: dict[str, tuple[str, ...]] = {}
        self.requirement_owner: dict[str, str] = {}
        self.impacts: dict[str, Impact] = {}

    def add(self, code: str, path: str, anchor: str, detail: str) -> None:
        self.diagnostics.append(Diagnostic(code, path, anchor, detail))

    def warn(self, code: str, path: str, anchor: str, detail: str) -> None:
        self.warnings.append(Diagnostic(code, path, anchor, detail))

    def closed_object(
        self,
        value: object,
        expected: set[str],
        path: str,
        anchor: str,
    ) -> bool:
        if not isinstance(value, dict):
            self.add("SCHEMA_TYPE_MISMATCH", path, anchor, "se esperaba un objeto")
            return False
        for field in sorted(set(value) - expected):
            self.add("SCHEMA_UNKNOWN_FIELD", path, anchor, f"campo desconocido: {field}")
        for field in sorted(expected - set(value)):
            self.add("SCHEMA_REQUIRED_FIELD", path, anchor, f"falta el campo: {field}")
        return True

    def relative_path(
        self,
        value: object,
        source_path: str,
        anchor: str,
        *,
        file_only: bool,
    ) -> pathlib.Path | None:
        if not isinstance(value, str) or not value:
            self.add("SCHEMA_TYPE_MISMATCH", source_path, anchor, "la ruta debe ser texto no vacío")
            return None
        candidate_path = pathlib.PurePosixPath(value)
        unsafe = (
            candidate_path.is_absolute()
            or "\\" in value
            or value != candidate_path.as_posix()
            or any(part in {"", ".", ".."} for part in candidate_path.parts)
        )
        if unsafe:
            self.add("ANCHOR_PATH_UNSAFE", source_path, anchor, f"ruta no segura: {value}")
            return None
        candidate = self.root.joinpath(*candidate_path.parts)
        try:
            resolved = candidate.resolve(strict=False)
            resolved.relative_to(self.root)
        except (OSError, ValueError):
            self.add("ANCHOR_PATH_UNSAFE", source_path, anchor, f"ruta fuera del repositorio: {value}")
            return None
        exists = candidate.is_file() if file_only else candidate.exists()
        if not exists:
            self.add("ANCHOR_PATH_MISSING", source_path, anchor, f"no existe: {value}")
            return None
        try:
            strict_resolved = candidate.resolve(strict=True)
            strict_resolved.relative_to(self.root)
        except (OSError, ValueError):
            self.add("ANCHOR_PATH_UNSAFE", source_path, anchor, f"el destino sale del repositorio: {value}")
            return None
        return candidate

    def ordered_strings(
        self,
        value: object,
        source_path: str,
        anchor: str,
        field: str,
        *,
        non_empty: bool = True,
    ) -> list[str] | None:
        if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
            self.add("SCHEMA_TYPE_MISMATCH", source_path, anchor, f"{field} debe ser una lista de texto")
            return None
        if non_empty and not value:
            self.add("SCHEMA_TYPE_MISMATCH", source_path, anchor, f"{field} no puede estar vacío")
        if value != sorted(value):
            self.add("ANCHOR_ORDER_INVALID", source_path, anchor, f"{field} no está ordenado")
        if len(value) != len(set(value)):
            self.add("ANCHOR_DUPLICATE_ID", source_path, anchor, f"{field} contiene duplicados")
        return value

    def selector_resolves(
        self,
        path: pathlib.Path,
        selector: str,
        source_path: str,
        anchor: str,
    ) -> None:
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeError):
            self.add("ANCHOR_PATH_MISSING", source_path, anchor, f"no se puede leer: {path.relative_to(self.root)}")
            return
        escaped = re.escape(selector)
        suffix = path.suffix.lower()
        if suffix == ".rs":
            found = re.search(rf"\bfn\s+{escaped}\s*\(", text) is not None
        elif suffix == ".py":
            found = re.search(rf"\b(?:async\s+)?def\s+{escaped}\s*\(", text) is not None
        elif suffix in {".sh", ".bash"}:
            found = re.search(rf"(?m)^\s*(?:function\s+)?{escaped}\s*(?:\(\s*\))?\s*\{{", text) is not None
        else:
            found = selector in text
        if not found:
            self.add(
                "ANCHOR_SELECTOR_UNRESOLVED",
                source_path,
                anchor,
                f"{path.relative_to(self.root).as_posix()} no declara {selector}",
            )

    def parse_specs(self) -> tuple[dict[str, list[str]], dict[str, list[tuple[str, str | None]]]]:
        capabilities: dict[str, list[str]] = {}
        requirements: dict[str, list[tuple[str, str | None]]] = {}
        for relative in SYSTEM_SPECS:
            path = self.root / relative
            if not path.is_file():
                self.add("ANCHOR_PATH_MISSING", relative, "spec", "falta una de las siete specs vivas")
                continue
            try:
                lines = path.read_text(encoding="utf-8").splitlines()
            except (OSError, UnicodeError):
                self.add("ANCHOR_PATH_MISSING", relative, "spec", "no se puede leer la spec viva")
                continue
            current_capability: str | None = None
            for line in lines:
                capability_match = CAPABILITY_HEADING.match(line)
                if capability_match:
                    current_capability = capability_match.group(1)
                    capabilities.setdefault(current_capability, []).append(relative)
                    continue
                requirement_match = REQUIREMENT_HEADING.match(line)
                if requirement_match:
                    requirement = requirement_match.group(1)
                    requirements.setdefault(requirement, []).append((relative, current_capability))
        return capabilities, requirements

    def parse_roadmap(self) -> dict[str, str]:
        path = self.root / ROADMAP_PATH
        if not path.is_file():
            self.add("ANCHOR_PATH_MISSING", ROADMAP_PATH.as_posix(), "roadmap", "falta ROADMAP.md")
            return {}
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except (OSError, UnicodeError):
            self.add("ANCHOR_PATH_MISSING", ROADMAP_PATH.as_posix(), "roadmap", "no se puede leer ROADMAP.md")
            return {}
        sections: dict[str, list[str]] = {}
        current: str | None = None
        for line in lines:
            match = ROADMAP_HEADING.match(line)
            if match:
                current = match.group(1)
                if current in sections:
                    self.add("ANCHOR_DUPLICATE_ID", ROADMAP_PATH.as_posix(), current, "entrada de roadmap duplicada")
                sections.setdefault(current, []).append(line)
            elif current is not None:
                sections[current].append(line)
        return {key: "\n".join(value) for key, value in sections.items()}

    def validate_verification(
        self,
        value: object,
        capability_id: str,
        requirement_id: str,
    ) -> str | None:
        source_path = REGISTRY_PATH.as_posix()
        if not self.closed_object(value, VERIFICATION_KEYS, source_path, requirement_id):
            return None
        assert isinstance(value, dict)
        kind = value.get("kind")
        if not isinstance(kind, str):
            self.add("SCHEMA_TYPE_MISMATCH", source_path, requirement_id, "kind debe ser texto")
            return None
        if kind not in KINDS:
            self.add("SCHEMA_UNKNOWN_ENUM", source_path, requirement_id, f"kind desconocido: {kind}")
            return None
        path = self.relative_path(value.get("path"), source_path, requirement_id, file_only=True)
        selector = value.get("selector")
        if kind == "manual_protocol":
            if selector is not None:
                self.add(
                    "SCHEMA_CONDITIONAL_INVALID",
                    source_path,
                    requirement_id,
                    "manual_protocol exige selector null",
                )
        elif not isinstance(selector, str) or not selector:
            self.add(
                "SCHEMA_CONDITIONAL_INVALID",
                source_path,
                requirement_id,
                f"{kind} exige selector no vacío",
            )
        elif path is not None:
            self.selector_resolves(path, selector, source_path, requirement_id)
        return kind

    def validate_requirement(
        self,
        value: object,
        capability_id: str,
    ) -> tuple[str | None, str | None]:
        source_path = REGISTRY_PATH.as_posix()
        anchor = capability_id
        if not self.closed_object(value, REQUIREMENT_KEYS, source_path, anchor):
            return None, None
        assert isinstance(value, dict)
        requirement_id = value.get("id")
        if not isinstance(requirement_id, str) or not REQUIREMENT_ID.fullmatch(requirement_id):
            self.add("SCHEMA_VALUE_INVALID", source_path, anchor, f"ID de requisito inválido: {requirement_id}")
            requirement_id = None
        elif not requirement_id.startswith(f"REQ-{capability_id.removeprefix('CAP-')}-"):
            self.add(
                "ANCHOR_OWNER_MISMATCH",
                source_path,
                requirement_id,
                f"el ID no pertenece a {capability_id}",
            )
        where = requirement_id or anchor
        statement = value.get("statement")
        if not isinstance(statement, str) or not statement.strip():
            self.add("SCHEMA_TYPE_MISMATCH", source_path, where, "statement debe ser texto no vacío")
        status = value.get("status")
        if not isinstance(status, str):
            self.add("SCHEMA_TYPE_MISMATCH", source_path, where, "status debe ser texto")
            status = None
        elif status not in STATUSES:
            self.add("SCHEMA_UNKNOWN_ENUM", source_path, where, f"status desconocido: {status}")
            status = None
        verifications = value.get("verifications")
        kinds: list[str] = []
        if not isinstance(verifications, list) or not verifications:
            self.add("SCHEMA_TYPE_MISMATCH", source_path, where, "verifications debe ser una lista no vacía")
        else:
            seen: set[str] = set()
            for verification in verifications:
                if isinstance(verification, dict):
                    key = json.dumps(
                        [
                            verification.get("kind"),
                            verification.get("path"),
                            verification.get("selector"),
                        ],
                        ensure_ascii=False,
                        sort_keys=True,
                    )
                    if key in seen:
                        self.add("ANCHOR_DUPLICATE_ID", source_path, where, "verificación duplicada")
                    seen.add(key)
                kind = self.validate_verification(verification, capability_id, where)
                if kind is not None:
                    kinds.append(kind)
        if status == "implemented" and not any(kind in {"test", "gate"} for kind in kinds):
            self.add(
                "ANCHOR_EXECUTABLE_REQUIRED",
                source_path,
                where,
                "implemented necesita al menos una prueba o gate ejecutable",
            )
        if status in {"partial", "external"} and "manual_protocol" not in kinds:
            self.add(
                "ANCHOR_FUTURE_WORK_REQUIRED",
                source_path,
                where,
                f"{status} necesita un protocolo manual",
            )
        return requirement_id, status

    def validate_capability(
        self,
        value: object,
        roadmap: dict[str, str],
    ) -> tuple[str | None, list[tuple[str | None, str | None]], str | None]:
        source_path = REGISTRY_PATH.as_posix()
        if not self.closed_object(value, CAPABILITY_KEYS, source_path, "capability"):
            return None, [], None
        assert isinstance(value, dict)
        capability_id = value.get("id")
        if not isinstance(capability_id, str) or not CAPABILITY_ID.fullmatch(capability_id):
            self.add("SCHEMA_VALUE_INVALID", source_path, "capability", f"ID inválido: {capability_id}")
            capability_id = None
        where = capability_id or "capability"
        title = value.get("title")
        if not isinstance(title, str) or not title.strip():
            self.add("SCHEMA_TYPE_MISMATCH", source_path, where, "title debe ser texto no vacío")
        owner_spec = value.get("owner_spec")
        if not isinstance(owner_spec, str) or not re.fullmatch(r"specs/system/[^/]+\.md", owner_spec):
            self.add("SCHEMA_VALUE_INVALID", source_path, where, f"owner_spec inválida: {owner_spec}")
            owner_spec = None
        elif owner_spec not in SYSTEM_SPECS:
            self.add("ANCHOR_OWNER_MISMATCH", source_path, where, f"owner_spec no es una spec viva: {owner_spec}")
        if owner_spec is not None:
            self.relative_path(owner_spec, source_path, where, file_only=True)

        code_paths = self.ordered_strings(value.get("code_paths"), source_path, where, "code_paths")
        if code_paths is not None:
            for relative in code_paths:
                self.relative_path(relative, source_path, where, file_only=False)
            pure_paths = [pathlib.PurePosixPath(relative) for relative in code_paths]
            for index, left in enumerate(pure_paths):
                for right in pure_paths[index + 1 :]:
                    if left == right or left.parts == right.parts[: len(left.parts)]:
                        self.add(
                            "ANCHOR_CODE_PATH_OVERLAP",
                            source_path,
                            where,
                            f"{left.as_posix()} solapa {right.as_posix()}",
                        )

        status = value.get("status")
        if not isinstance(status, str):
            self.add("SCHEMA_TYPE_MISMATCH", source_path, where, "status debe ser texto")
            status = None
        elif status not in STATUSES:
            self.add("SCHEMA_UNKNOWN_ENUM", source_path, where, f"status desconocido: {status}")
            status = None
        requirements_value = value.get("requirements")
        requirements: list[tuple[str | None, str | None]] = []
        if not isinstance(requirements_value, list) or not requirements_value:
            self.add("SCHEMA_TYPE_MISMATCH", source_path, where, "requirements debe ser una lista no vacía")
        else:
            requirements = [self.validate_requirement(item, where) for item in requirements_value]
            ids = [item[0] for item in requirements if item[0] is not None]
            if ids != sorted(ids):
                self.add("ANCHOR_ORDER_INVALID", source_path, where, "requirements no está ordenado")

        evidence = self.ordered_strings(value.get("evidence"), source_path, where, "evidence")
        if evidence is not None:
            for relative in evidence:
                self.relative_path(relative, source_path, where, file_only=True)

        roadmap_id = value.get("roadmap_id")
        if roadmap_id is not None and (
            not isinstance(roadmap_id, str) or not ROADMAP_ID.fullmatch(roadmap_id)
        ):
            self.add("SCHEMA_VALUE_INVALID", source_path, where, f"roadmap_id inválido: {roadmap_id}")
            roadmap_id = None
        protocol_value = value.get("protocol")
        protocol_path: pathlib.Path | None = None
        if protocol_value is not None:
            protocol_path = self.relative_path(protocol_value, source_path, where, file_only=True)

        if status in {"partial", "external"} and (
            not isinstance(roadmap_id, str) or not isinstance(protocol_value, str)
        ):
            self.add(
                "ANCHOR_FUTURE_WORK_REQUIRED",
                source_path,
                where,
                f"{status} exige roadmap_id y protocol",
            )
        if status == "deprecated" and not isinstance(roadmap_id, str):
            self.add(
                "ANCHOR_FUTURE_WORK_REQUIRED",
                source_path,
                where,
                "deprecated exige roadmap de retirada",
            )
        if status in {"implemented", "deprecated"} and protocol_value is not None:
            self.add(
                "SCHEMA_CONDITIONAL_INVALID",
                source_path,
                where,
                f"{status} exige protocol null",
            )
        if isinstance(roadmap_id, str) and roadmap_id not in roadmap:
            self.add("ANCHOR_PATH_MISSING", ROADMAP_PATH.as_posix(), roadmap_id, "entrada no declarada")

        requirement_statuses = [item[1] for item in requirements]
        if status is not None and requirement_statuses and all(item in STATUSES for item in requirement_statuses):
            if all(item == "implemented" for item in requirement_statuses):
                derived = "implemented"
            elif all(item == "external" for item in requirement_statuses):
                derived = "external"
            elif all(item == "deprecated" for item in requirement_statuses):
                derived = "deprecated"
            elif (
                any(item in {"implemented", "partial"} for item in requirement_statuses)
                and any(item in {"partial", "external"} for item in requirement_statuses)
                and "deprecated" not in requirement_statuses
            ):
                derived = "partial"
            else:
                derived = "invalid"
            if status != derived:
                self.add(
                    "ANCHOR_STATUS_MISMATCH",
                    source_path,
                    where,
                    f"status {status}; derivado {derived}",
                )

        future_requirements = [
            requirement_id
            for requirement_id, requirement_status in requirements
            if requirement_id is not None and requirement_status in {"partial", "external", "deprecated"}
        ]
        if isinstance(roadmap_id, str) and roadmap_id in roadmap:
            for requirement_id in future_requirements:
                if requirement_id not in roadmap[roadmap_id]:
                    self.add(
                        "ANCHOR_FUTURE_WORK_REQUIRED",
                        ROADMAP_PATH.as_posix(),
                        requirement_id,
                        f"{roadmap_id} no enumera el requisito",
                    )
        if protocol_path is not None:
            try:
                protocol_text = protocol_path.read_text(encoding="utf-8")
            except (OSError, UnicodeError):
                protocol_text = ""
            for requirement_id in [
                requirement_id
                for requirement_id, requirement_status in requirements
                if requirement_id is not None and requirement_status in {"partial", "external"}
            ]:
                if requirement_id not in protocol_text:
                    self.add(
                        "ANCHOR_FUTURE_WORK_REQUIRED",
                        str(protocol_value),
                        requirement_id,
                        "el protocolo no cubre el requisito",
                    )
        return capability_id, requirements, owner_spec

    @staticmethod
    def non_empty_string(value: object) -> bool:
        return isinstance(value, str) and bool(value.strip())

    def impact_strings(
        self,
        value: object,
        source_path: str,
        feature_id: str,
        field: str,
        pattern: re.Pattern[str] | None = None,
    ) -> list[str] | None:
        if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
            self.add("SCHEMA_TYPE_MISMATCH", source_path, feature_id, f"{field} debe ser una lista de texto")
            return None
        if value != sorted(value):
            self.add("SCHEMA_VALUE_INVALID", source_path, feature_id, f"{field} no está ordenado")
        if len(value) != len(set(value)):
            self.add("SCHEMA_VALUE_INVALID", source_path, feature_id, f"{field} contiene duplicados")
        if pattern is not None:
            for item in value:
                if not pattern.fullmatch(item):
                    self.add("SCHEMA_VALUE_INVALID", source_path, feature_id, f"{field} contiene ID inválido: {item}")
        return value

    def validate_impact(self, path: pathlib.Path) -> Impact | None:
        source_path = path.relative_to(self.root).as_posix()
        before = len(self.diagnostics)
        try:
            value = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError) as error:
            self.add("SCHEMA_VALUE_INVALID", source_path, "impact", f"JSON inválido: {error}")
            return None
        if not self.closed_object(value, IMPACT_KEYS, source_path, "impact"):
            return None
        assert isinstance(value, dict)

        if type(value.get("schema_version")) is not int or value.get("schema_version") != 1:
            self.add("SCHEMA_VALUE_INVALID", source_path, "impact", "schema_version debe ser 1")

        feature_value = value.get("feature_id")
        feature_id = feature_value if isinstance(feature_value, str) else "impact"
        if not isinstance(feature_value, str) or not FEATURE_ID.fullmatch(feature_value):
            self.add("SCHEMA_VALUE_INVALID", source_path, feature_id, f"feature_id inválido: {feature_value}")
        elif feature_value != path.parent.name:
            self.add(
                "SCHEMA_VALUE_INVALID",
                source_path,
                feature_id,
                f"feature_id no coincide con el directorio {path.parent.name}",
            )

        change_value = value.get("change_type")
        change_type = change_value if isinstance(change_value, str) else ""
        if not isinstance(change_value, str):
            self.add("SCHEMA_TYPE_MISMATCH", source_path, feature_id, "change_type debe ser texto")
        elif change_value not in CHANGE_TYPES:
            self.add("SCHEMA_UNKNOWN_ENUM", source_path, feature_id, f"change_type desconocido: {change_value}")

        capabilities = self.impact_strings(
            value.get("capabilities"), source_path, feature_id, "capabilities", CAPABILITY_ID
        )
        requirements = self.impact_strings(
            value.get("requirements"), source_path, feature_id, "requirements", REQUIREMENT_ID
        )
        if capabilities is not None:
            for capability in capabilities:
                if CAPABILITY_ID.fullmatch(capability) and capability not in self.capability_paths:
                    self.add("SCHEMA_VALUE_INVALID", source_path, feature_id, f"capacidad desconocida: {capability}")
        if requirements is not None:
            for requirement in requirements:
                if REQUIREMENT_ID.fullmatch(requirement) and requirement not in self.requirement_owner:
                    self.add("SCHEMA_VALUE_INVALID", source_path, feature_id, f"requisito desconocido: {requirement}")
                elif requirement in self.requirement_owner and capabilities is not None:
                    owner = self.requirement_owner[requirement]
                    if owner not in capabilities:
                        self.add(
                            "SCHEMA_VALUE_INVALID",
                            source_path,
                            feature_id,
                            f"{requirement} pertenece a una capacidad no declarada: {owner}",
                        )

        compatibility = value.get("compatibility")
        compatibility_values: list[object] = []
        if self.closed_object(compatibility, COMPATIBILITY_KEYS, source_path, feature_id):
            assert isinstance(compatibility, dict)
            for field in ("public_contract", "persisted_data"):
                verdict = compatibility.get(field)
                compatibility_values.append(verdict)
                if not isinstance(verdict, str):
                    self.add("SCHEMA_TYPE_MISMATCH", source_path, feature_id, f"compatibility.{field} debe ser texto")
                elif verdict not in COMPATIBILITY_VERDICTS:
                    self.add("SCHEMA_UNKNOWN_ENUM", source_path, feature_id, f"compatibility.{field} desconocido: {verdict}")
            if not self.non_empty_string(compatibility.get("notes")):
                self.add("SCHEMA_TYPE_MISMATCH", source_path, feature_id, "compatibility.notes debe ser texto no vacío")

        migration = value.get("migration")
        migration_required: bool | None = None
        if self.closed_object(migration, MIGRATION_KEYS, source_path, feature_id):
            assert isinstance(migration, dict)
            required_value = migration.get("required")
            if type(required_value) is not bool:
                self.add("SCHEMA_TYPE_MISMATCH", source_path, feature_id, "migration.required debe ser booleano")
            else:
                migration_required = required_value
                recovery = [migration.get(field) for field in ("plan", "backup", "retry")]
                valid_recovery = all(self.non_empty_string(item) for item in recovery)
                null_recovery = all(item is None for item in recovery)
                if (required_value and not valid_recovery) or (not required_value and not null_recovery):
                    self.add(
                        "SCHEMA_CONDITIONAL_INVALID",
                        source_path,
                        feature_id,
                        "migration debe usar tres textos recuperables si required=true y tres null si es false",
                    )
        if "incompatible" in compatibility_values and migration_required is not True:
            self.add(
                "SCHEMA_CONDITIONAL_INVALID",
                source_path,
                feature_id,
                "una incompatibilidad exige migration.required=true",
            )

        rollback = value.get("rollback")
        if self.closed_object(rollback, ROLLBACK_KEYS, source_path, feature_id):
            assert isinstance(rollback, dict)
            strategy = rollback.get("strategy")
            if not isinstance(strategy, str):
                self.add("SCHEMA_TYPE_MISMATCH", source_path, feature_id, "rollback.strategy debe ser texto")
            elif strategy not in ROLLBACK_STRATEGIES:
                self.add("SCHEMA_UNKNOWN_ENUM", source_path, feature_id, f"rollback.strategy desconocido: {strategy}")
            elif strategy == "not_applicable":
                if rollback.get("procedure") is not None or rollback.get("success_check") is not None:
                    self.add(
                        "SCHEMA_CONDITIONAL_INVALID",
                        source_path,
                        feature_id,
                        "rollback not_applicable exige procedure y success_check null",
                    )
                if change_type and change_type != "docs_only":
                    self.add(
                        "SCHEMA_CONDITIONAL_INVALID",
                        source_path,
                        feature_id,
                        "rollback not_applicable sólo se admite para docs_only",
                    )
            elif strategy in ROLLBACK_STRATEGIES and not all(
                self.non_empty_string(rollback.get(field)) for field in ("procedure", "success_check")
            ):
                self.add(
                    "SCHEMA_CONDITIONAL_INVALID",
                    source_path,
                    feature_id,
                    "rollback recuperable exige procedure y success_check no vacíos",
                )

        living_specs_updated = value.get("living_specs_updated")
        if type(living_specs_updated) is not bool:
            self.add("SCHEMA_TYPE_MISMATCH", source_path, feature_id, "living_specs_updated debe ser booleano")
        elif change_type in {"behavior", "contract"} and not living_specs_updated:
            self.add(
                "IMPACT_LIVING_SPEC_REQUIRED",
                source_path,
                feature_id,
                f"{change_type} exige living_specs_updated=true",
            )
        elif change_type in {"internal_refactor", "docs_only"} and living_specs_updated:
            self.add(
                "SCHEMA_CONDITIONAL_INVALID",
                source_path,
                feature_id,
                f"{change_type} exige living_specs_updated=false",
            )

        characterization = self.impact_strings(
            value.get("characterization"), source_path, feature_id, "characterization"
        )
        if change_type == "internal_refactor" and not characterization:
            self.add(
                "IMPACT_CHARACTERIZATION_REQUIRED",
                source_path,
                feature_id,
                "internal_refactor exige caracterización no vacía",
            )
        if characterization is not None:
            for relative in characterization:
                if self.relative_path(relative, source_path, feature_id, file_only=True) is None:
                    self.add(
                        "IMPACT_CHARACTERIZATION_REQUIRED",
                        source_path,
                        feature_id,
                        f"caracterización no resoluble: {relative}",
                    )

        if len(self.diagnostics) != before:
            return None
        assert isinstance(feature_value, str)
        assert capabilities is not None and requirements is not None
        return Impact(
            path=source_path,
            feature_id=feature_value,
            change_type=change_type,
            capabilities=tuple(capabilities),
            requirements=tuple(requirements),
        )

    def validate_impacts(self) -> None:
        self.relative_path(
            IMPACT_SCHEMA_PATH.as_posix(),
            IMPACT_SCHEMA_PATH.as_posix(),
            "FeatureImpactV1",
            file_only=True,
        )
        pattern = self.root / "specs"
        if not pattern.is_dir():
            return
        for path in sorted(pattern.glob("[0-9][0-9][0-9]-*/feature-impact.json")):
            impact = self.validate_impact(path)
            if impact is not None:
                self.impacts[impact.path] = impact

    @staticmethod
    def path_is_within(path: str, prefix: str) -> bool:
        path_parts = pathlib.PurePosixPath(path).parts
        prefix_parts = pathlib.PurePosixPath(prefix).parts
        return path_parts[: len(prefix_parts)] == prefix_parts

    def normative_paths(self) -> set[str]:
        result: set[str] = set()
        path = self.root / CLASSIFICATION_PATH
        if not path.is_file():
            return result
        try:
            text = path.read_text(encoding="utf-8")
        except (OSError, UnicodeError):
            return result
        row = re.compile(r"^\| `([^`]+)` \| `normative` \|", re.MULTILINE)
        return set(row.findall(text))

    def correlate_diff(self, changed_paths: set[str], *, report: bool) -> None:
        active = [self.impacts[path] for path in sorted(changed_paths & set(self.impacts))]
        active_capabilities = {capability for impact in active for capability in impact.capabilities}
        active_requirements = {requirement for impact in active for requirement in impact.requirements}
        path_capabilities: dict[str, set[str]] = {}
        for changed in sorted(changed_paths):
            path_capabilities[changed] = {
                capability
                for capability, prefixes in self.capability_paths.items()
                if any(self.path_is_within(changed, prefix) for prefix in prefixes)
            }

        def emit(code: str, path: str, anchor: str, detail: str) -> None:
            if report:
                self.warn(code, path, anchor, detail)
            else:
                self.add(code, path, anchor, detail)

        governed = {
            path
            for path, capabilities in path_capabilities.items()
            if capabilities or self.path_is_within(path, "crates")
        }
        if governed and not active:
            for path in sorted(governed):
                emit("IMPACT_REQUIRED", path, "diff", "cambio de producto sin paquete de impacto activo")
        elif active:
            for path in sorted(governed):
                capabilities = path_capabilities[path]
                if self.path_is_within(path, "crates") and not capabilities:
                    emit("IMPACT_REQUIRED", path, "diff", "ruta de producto sin capacidad registrada")
                for capability in sorted(capabilities - active_capabilities):
                    emit("IMPACT_REQUIRED", path, capability, "capacidad afectada ausente del impacto activo")
                for capability in sorted(capabilities & active_capabilities):
                    if not any(
                        self.requirement_owner.get(requirement) == capability
                        for requirement in active_requirements
                    ):
                        emit(
                            "IMPACT_REQUIRED",
                            path,
                            capability,
                            "la capacidad afectada no declara ningún requisito propio",
                        )

        classified_normative = self.normative_paths()
        normative = {
            path
            for path in changed_paths
            if path in classified_normative
            or self.path_is_within(path, "specs/system")
            or self.path_is_within(path, "specs/schemas")
        }
        if normative and not active:
            for path in sorted(normative):
                emit("IMPACT_REQUIRED", path, "diff", "cambio normativo sin paquete de impacto activo")
        elif normative and not any(impact.change_type == "contract" for impact in active):
            for path in sorted(normative):
                emit(
                    "IMPACT_CHANGE_TYPE_CONTRACT",
                    path,
                    "diff",
                    "una autoridad normativa modificada exige change_type=contract",
                )

    def run(self) -> None:
        registry_file = self.root / REGISTRY_PATH
        if not registry_file.is_file():
            self.add("ANCHOR_PATH_MISSING", REGISTRY_PATH.as_posix(), "registry", "falta el registro")
            return
        try:
            registry = json.loads(registry_file.read_text(encoding="utf-8"))
        except (OSError, UnicodeError, json.JSONDecodeError) as error:
            self.add("ANCHOR_JSON_INVALID", REGISTRY_PATH.as_posix(), "registry", str(error))
            return
        if not self.closed_object(registry, ROOT_KEYS, REGISTRY_PATH.as_posix(), "registry"):
            return
        assert isinstance(registry, dict)
        if registry.get("schema_version") != 1:
            self.add("SCHEMA_VALUE_INVALID", REGISTRY_PATH.as_posix(), "registry", "schema_version debe ser 1")
        if registry.get("baseline_commit") != BASELINE:
            self.add("SCHEMA_VALUE_INVALID", REGISTRY_PATH.as_posix(), "registry", "baseline_commit no es K4")
        if registry.get("generated_from") != GENERATED_FROM:
            self.add("SCHEMA_VALUE_INVALID", REGISTRY_PATH.as_posix(), "registry", f"generated_from debe ser {GENERATED_FROM}")

        roadmap = self.parse_roadmap()
        capabilities_value = registry.get("capabilities")
        if not isinstance(capabilities_value, list) or not capabilities_value:
            self.add("SCHEMA_TYPE_MISMATCH", REGISTRY_PATH.as_posix(), "registry", "capabilities debe ser una lista no vacía")
            return
        parsed = [self.validate_capability(item, roadmap) for item in capabilities_value]
        for raw, (capability_id, requirements, _) in zip(capabilities_value, parsed, strict=True):
            if capability_id is None or not isinstance(raw, dict):
                continue
            raw_paths = raw.get("code_paths")
            if isinstance(raw_paths, list) and all(isinstance(item, str) for item in raw_paths):
                self.capability_paths[capability_id] = tuple(raw_paths)
            for requirement_id, _ in requirements:
                if requirement_id is not None:
                    self.requirement_owner[requirement_id] = capability_id
        capability_ids = [item[0] for item in parsed if item[0] is not None]
        requirement_ids = [
            requirement_id
            for _, requirements, _ in parsed
            for requirement_id, _ in requirements
            if requirement_id is not None
        ]
        self.capability_count = len(capability_ids)
        self.requirement_count = len(requirement_ids)
        if capability_ids != sorted(capability_ids):
            self.add("ANCHOR_ORDER_INVALID", REGISTRY_PATH.as_posix(), "capabilities", "capabilities no está ordenado")
        for identifier, count in sorted(Counter(capability_ids).items()):
            if count > 1:
                self.add("ANCHOR_DUPLICATE_ID", REGISTRY_PATH.as_posix(), identifier, "ID de capacidad duplicado")
        for identifier, count in sorted(Counter(requirement_ids).items()):
            if count > 1:
                self.add("ANCHOR_DUPLICATE_ID", REGISTRY_PATH.as_posix(), identifier, "ID de requisito duplicado")

        spec_capabilities, spec_requirements = self.parse_specs()
        for identifier, locations in sorted(spec_capabilities.items()):
            if len(locations) > 1:
                self.add("ANCHOR_DUPLICATE_ID", locations[0], identifier, "capacidad declarada más de una vez")
        for identifier, locations in sorted(spec_requirements.items()):
            if len(locations) > 1:
                self.add("ANCHOR_DUPLICATE_ID", locations[0][0], identifier, "requisito declarado más de una vez")
        if set(capability_ids) != set(spec_capabilities):
            missing = sorted(set(spec_capabilities) - set(capability_ids))
            extra = sorted(set(capability_ids) - set(spec_capabilities))
            self.add(
                "ANCHOR_COVERAGE_MISMATCH",
                REGISTRY_PATH.as_posix(),
                "capabilities",
                f"sólo en specs={missing}; sólo en anchors={extra}",
            )
        if set(requirement_ids) != set(spec_requirements):
            missing = sorted(set(spec_requirements) - set(requirement_ids))
            extra = sorted(set(requirement_ids) - set(spec_requirements))
            self.add(
                "ANCHOR_COVERAGE_MISMATCH",
                REGISTRY_PATH.as_posix(),
                "requirements",
                f"sólo en specs={missing}; sólo en anchors={extra}",
            )
        for capability_id, requirements, owner_spec in parsed:
            if capability_id is None or owner_spec is None:
                continue
            declarations = spec_capabilities.get(capability_id, [])
            if declarations != [owner_spec]:
                self.add(
                    "ANCHOR_OWNER_MISMATCH",
                    owner_spec,
                    capability_id,
                    f"declaraciones encontradas: {declarations}",
                )
            for requirement_id, _ in requirements:
                if requirement_id is None:
                    continue
                declarations_for_requirement = spec_requirements.get(requirement_id, [])
                if declarations_for_requirement != [(owner_spec, capability_id)]:
                    self.add(
                        "ANCHOR_OWNER_MISMATCH",
                        owner_spec,
                        requirement_id,
                        f"declaraciones encontradas: {declarations_for_requirement}",
                    )
        self.validate_impacts()


def parse_arguments(arguments: list[str]) -> tuple[str | None, bool, Diagnostic | None]:
    base: str | None = None
    report = False
    index = 0
    while index < len(arguments):
        argument = arguments[index]
        if argument == "--base":
            if base is not None or index + 1 >= len(arguments) or arguments[index + 1].startswith("--"):
                return None, False, Diagnostic(
                    "USAGE_INVALID",
                    "scripts_ci/validate_spec_anchors.py",
                    "arguments",
                    "--base exige exactamente una referencia Git",
                )
            base = arguments[index + 1]
            index += 2
            continue
        if argument == "--report":
            if report:
                return None, False, Diagnostic(
                    "USAGE_INVALID",
                    "scripts_ci/validate_spec_anchors.py",
                    "arguments",
                    "--report no puede repetirse",
                )
            report = True
            index += 1
            continue
        return None, False, Diagnostic(
            "USAGE_INVALID",
            "scripts_ci/validate_spec_anchors.py",
            "arguments",
            f"argumento desconocido: {argument}",
        )
    return base, report, None


def resolve_base(root: pathlib.Path, reference: str) -> str | None:
    result = subprocess.run(
        ["git", "rev-parse", "--verify", "--quiet", "--end-of-options", f"{reference}^{{commit}}"],
        cwd=root,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        return None
    resolved = result.stdout.decode("ascii", errors="ignore").strip()
    return resolved if re.fullmatch(r"[0-9a-fA-F]{40,64}", resolved) else None


def git_changed_paths(root: pathlib.Path, base: str) -> set[str] | None:
    result = subprocess.run(
        [
            "git",
            "diff",
            "--name-status",
            "-z",
            "--find-renames",
            f"{base}...HEAD",
            "--",
        ],
        cwd=root,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        return None
    fields = result.stdout.split(b"\0")
    if fields and fields[-1] == b"":
        fields.pop()
    paths: set[str] = set()
    index = 0
    try:
        while index < len(fields):
            status = fields[index].decode("ascii")
            index += 1
            count = 2 if status[:1] in {"R", "C"} else 1
            for _ in range(count):
                paths.add(fields[index].decode("utf-8"))
                index += 1
    except (IndexError, UnicodeDecodeError):
        return None
    return paths


def main(argv: list[str] | None = None) -> int:
    arguments = sys.argv[1:] if argv is None else argv
    cli_base, report, usage_error = parse_arguments(arguments)
    if usage_error is not None:
        print(usage_error.render(), file=sys.stderr)
        return 2
    if not ROOT.is_dir():
        diagnostic = Diagnostic(
            "ANCHOR_ROOT_UNREADABLE",
            ".",
            "root",
            "no se puede leer la raíz del repositorio",
        )
        print(diagnostic.render(), file=sys.stderr)
        return 2

    base = cli_base if cli_base is not None else os.environ.get("BATUTA_SPEC_BASE") or None
    resolved_base: str | None = None
    if base is not None:
        resolved_base = resolve_base(ROOT, base)
        if resolved_base is None:
            diagnostic = Diagnostic(
                "GIT_BASE_UNRESOLVABLE",
                ".",
                "git",
                base,
            )
            print(diagnostic.render(), file=sys.stderr)
            return 2

    validation = Validation(ROOT)
    validation.run()
    if resolved_base is None:
        validation.warn(
            "GIT_DIFF_OMITTED",
            ".",
            "git",
            "base Git no proporcionada; correlación de diff omitida",
        )
    else:
        changed_paths = git_changed_paths(ROOT, resolved_base)
        if changed_paths is None:
            diagnostic = Diagnostic("GIT_BASE_UNRESOLVABLE", ".", "git", base or resolved_base)
            print(diagnostic.render(), file=sys.stderr)
            return 2
        validation.correlate_diff(changed_paths, report=report)

    diagnostics = sorted(set(validation.diagnostics))
    output = sorted(set(diagnostics + validation.warnings))
    for diagnostic in output:
        print(diagnostic.render(), file=sys.stderr)
    if diagnostics:
        return 1
    print(
        f"validated {validation.capability_count} capabilities and "
        f"{validation.requirement_count} requirements"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
