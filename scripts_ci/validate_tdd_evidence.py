#!/usr/bin/env python3
"""Validate Batuta's red/green/mutation JSONL evidence without third-party deps."""

from __future__ import annotations

import hashlib
import json
import pathlib
import re
import sys
from typing import NoReturn


ROOT = pathlib.Path(__file__).resolve().parents[1]
V1_LOG = ROOT / "docs" / "evidence" / "tdd.jsonl"
V1_SCHEMA = ROOT / "docs" / "evidence" / "tdd.schema.json"
V1_BASELINE = ROOT / "docs" / "evidence" / "v1-baseline.json"
V2_LOG = ROOT / "docs" / "evidence" / "tdd-v2.jsonl"
V2_SCHEMA = ROOT / "specs" / "schemas" / "evidence-record-v2.schema.json"
LOG = V1_LOG
SCHEMA = V1_SCHEMA
LEGACY_TASK_ID = re.compile(r"^K[0-7]\.\d+$")
FEATURE_ID = re.compile(r"^[0-9]{3}-[a-z0-9]+(?:-[a-z0-9]+)*$")
TASK_ID = re.compile(r"^T[0-9]{3}$")
REQUIREMENT_ID = re.compile(r"^REQ-[A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*-[0-9]{3}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
UTC_TIMESTAMP = re.compile(
    r"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(?:\.[0-9]+)?Z$"
)
LEGACY_REQUIRED = {
    "task_id",
    "evidence_mode",
    "spec_paths",
    "spec_snapshot",
    "spec_sha256",
    "red",
    "green",
    "mutation",
    "recorded_at",
}
V2_REQUIRED = {
    "schema_version",
    "feature_id",
    "task_id",
    "requirement_ids",
    "evidence_mode",
    "spec_snapshot",
    "red",
    "green",
    "mutation",
    "recorded_at",
}
EXPECTED_V1_ARTIFACTS = {
    "docs/evidence/specs/59ddd6234aee1a95fc7db4ecfaeee0ced3befe140190f305f21e00b0f42139f7.md",
    "docs/evidence/specs/8d1e228e24c449136102608028b2b37403c4624529712d4e00ceba2979999042.md",
    "docs/evidence/specs/9274306a9ad4a83ad9e061e4617d7e547e62f841ebd8c5373a554b17e812a70a.md",
    "docs/evidence/specs/b4ef1a975e590d84f6b29b5787139f1aea3cd7c2d82190c774e6e567c9d42872.md",
    "docs/evidence/tdd.jsonl",
    "docs/evidence/tdd.schema.json",
}
EXPECTED_V1_RUN_RECORDS = {"docs/evidence/baseline.json"}


class EvidenceError(ValueError):
    """Stable repository-relative validation diagnostic."""

    def __init__(self, code: str, path: str, detail: str) -> None:
        super().__init__(f"[{code}] {path}: {detail}")


def reject(code: str, path: str, detail: str) -> NoReturn:
    raise EvidenceError(code, path, detail)


def fail(message: str) -> None:
    raise ValueError(message)


def validate_command(value: object, expected_exit: int | None, where: str) -> None:
    if not isinstance(value, dict):
        fail(f"{where} must be an object")
    if set(value) != {"command", "exit_code", "summary"}:
        fail(f"{where} must contain exactly command, exit_code and summary")
    if not isinstance(value["command"], str) or not value["command"].strip():
        fail(f"{where}.command must be non-empty")
    if not isinstance(value["exit_code"], int) or value["exit_code"] < 0:
        fail(f"{where}.exit_code must be a non-negative integer")
    if expected_exit is not None and value["exit_code"] != expected_exit:
        fail(f"{where}.exit_code must be {expected_exit}")
    if not isinstance(value["summary"], str) or not value["summary"].strip():
        fail(f"{where}.summary must be non-empty")


def validate_mutation(value: object, where: str) -> None:
    if not isinstance(value, dict):
        fail(f"{where} must be an object")
    if set(value) != {"command", "exit_code", "summary", "killed"}:
        fail(f"{where} must contain command, exit_code, summary and killed")
    command = {key: value[key] for key in ("command", "exit_code", "summary")}
    validate_command(command, None, where)
    if value["killed"] is not True:
        fail(f"{where}.killed must be true")


def validate_legacy_record(
    record: object, line_number: int, root: pathlib.Path = ROOT
) -> None:
    where = f"line {line_number}"
    if not isinstance(record, dict):
        fail(f"{where} must be a JSON object")
    missing = LEGACY_REQUIRED - set(record)
    if missing:
        fail(f"{where} is missing {sorted(missing)}")
    extra = set(record) - LEGACY_REQUIRED
    if extra:
        fail(f"{where} has unknown fields {sorted(extra)}")
    if not isinstance(record["task_id"], str) or not LEGACY_TASK_ID.fullmatch(
        record["task_id"]
    ):
        fail(f"{where}.task_id must match K0.1..K7.n")
    mode = record["evidence_mode"]
    if mode not in {"tdd", "reconstructed_audit"}:
        fail(f"{where}.evidence_mode must be tdd or reconstructed_audit")
    paths = record["spec_paths"]
    if not isinstance(paths, list) or not paths or not all(isinstance(p, str) for p in paths):
        fail(f"{where}.spec_paths must be a non-empty string array")
    if len(paths) != len(set(paths)):
        fail(f"{where}.spec_paths must not contain duplicates")
    for relative in paths:
        path = root / relative
        if not path.is_file():
            fail(f"{where} references missing SPEC {relative}")
    snapshot = record["spec_snapshot"]
    if not isinstance(snapshot, str) or not re.fullmatch(
        r"docs/evidence/specs/[0-9a-f]{64}\.md", snapshot
    ):
        fail(f"{where}.spec_snapshot must be content-addressed")
    snapshot_path = root / snapshot
    if not snapshot_path.is_file():
        fail(f"{where} references missing SPEC snapshot {snapshot}")
    digest = hashlib.sha256(snapshot_path.read_bytes()).hexdigest()
    filename_digest = snapshot_path.stem
    if not isinstance(record["spec_sha256"], str) or not SHA256.fullmatch(
        record["spec_sha256"]
    ):
        fail(f"{where}.spec_sha256 must be lowercase SHA-256")
    if record["spec_sha256"] != digest or filename_digest != digest:
        fail(f"{where}.spec_sha256 does not match immutable snapshot bytes")
    validate_command(record["red"], None, f"{where}.red")
    if mode == "tdd" and record["red"]["exit_code"] == 0:
        fail(f"{where}.red must record a failing command")
    if mode == "reconstructed_audit" and "reconstruct" not in record["red"][
        "summary"
    ].lower():
        fail(f"{where}.red must explain the reconstructed audit")
    validate_command(record["green"], 0, f"{where}.green")
    validate_mutation(record["mutation"], f"{where}.mutation")
    if not isinstance(record["recorded_at"], str) or not record["recorded_at"].endswith("Z"):
        fail(f"{where}.recorded_at must be an explicit UTC timestamp")


validate_record = validate_legacy_record


def _load_json(path: pathlib.Path, code: str, display: str) -> object:
    if not path.is_file():
        reject(code, display, "ruta ausente")
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        reject(code, display, f"JSON inválido: {error}")


def _resolve_file(
    root: pathlib.Path, relative: str, code: str, detail: str
) -> pathlib.Path:
    pure = pathlib.PurePosixPath(relative)
    if (
        not relative
        or "\\" in relative
        or pure.is_absolute()
        or any(part in {"", ".", ".."} for part in pure.parts)
    ):
        reject(code, relative or ".", "ruta relativa insegura")
    candidate = root.joinpath(*pure.parts)
    try:
        candidate.resolve(strict=False).relative_to(root.resolve())
    except ValueError:
        reject(code, relative, "ruta fuera del repositorio")
    if not candidate.is_file():
        reject(code, relative, detail)
    return candidate


def _validate_manifest_entry(value: object, where: str) -> dict[str, object]:
    if not isinstance(value, dict) or set(value) != {"path", "bytes", "sha256"}:
        reject(
            "EVIDENCE_V1_MANIFEST_INVALID",
            where,
            "cada entrada debe contener exactamente path, bytes y sha256",
        )
    relative = value["path"]
    size = value["bytes"]
    digest = value["sha256"]
    if not isinstance(relative, str) or not relative:
        reject("EVIDENCE_V1_MANIFEST_INVALID", where, "path debe ser texto no vacío")
    if type(size) is not int or size < 0:
        reject("EVIDENCE_V1_MANIFEST_INVALID", where, "bytes debe ser entero no negativo")
    if not isinstance(digest, str) or not SHA256.fullmatch(digest):
        reject("EVIDENCE_V1_MANIFEST_INVALID", where, "sha256 debe ser hexadecimal en minúsculas")
    return value


def validate_v1_manifest(root: pathlib.Path) -> tuple[dict[str, object], list[str]]:
    display = "docs/evidence/v1-baseline.json"
    value = _load_json(root / display, "EVIDENCE_V1_MANIFEST_INVALID", display)
    required = {
        "schema_version",
        "baseline_commit",
        "record_count",
        "artifacts",
        "run_records",
    }
    if not isinstance(value, dict) or set(value) != required:
        reject(
            "EVIDENCE_V1_MANIFEST_INVALID",
            display,
            "claves raíz distintas del contrato cerrado",
        )
    if value["schema_version"] != 1:
        reject("EVIDENCE_V1_MANIFEST_INVALID", display, "schema_version debe ser 1")
    if value["baseline_commit"] != "7de68af2c9a36ba3dcc65971e4bba83231fb3855":
        reject("EVIDENCE_V1_MANIFEST_INVALID", display, "baseline_commit inesperado")
    if value["record_count"] != 19:
        reject("EVIDENCE_V1_RECORD_COUNT", display, "record_count debe permanecer en 19")
    if not isinstance(value["artifacts"], list) or not isinstance(
        value["run_records"], list
    ):
        reject("EVIDENCE_V1_MANIFEST_INVALID", display, "artifacts y run_records deben ser listas")

    artifacts = [
        _validate_manifest_entry(entry, f"{display}#artifacts[{index}]")
        for index, entry in enumerate(value["artifacts"])
    ]
    run_records = [
        _validate_manifest_entry(entry, f"{display}#run_records[{index}]")
        for index, entry in enumerate(value["run_records"])
    ]
    artifact_paths = [entry["path"] for entry in artifacts]
    run_paths = [entry["path"] for entry in run_records]
    if len(artifacts) != 6 or set(artifact_paths) != EXPECTED_V1_ARTIFACTS:
        reject(
            "EVIDENCE_V1_MANIFEST_INVALID",
            display,
            "artifacts debe enumerar exactamente los seis artefactos V1",
        )
    if len(run_records) != 1 or set(run_paths) != EXPECTED_V1_RUN_RECORDS:
        reject(
            "EVIDENCE_V1_MANIFEST_INVALID",
            display,
            "run_records debe enumerar exactamente el registro de corrida",
        )
    if artifact_paths != sorted(artifact_paths) or run_paths != sorted(run_paths):
        reject("EVIDENCE_V1_MANIFEST_INVALID", display, "las rutas deben estar ordenadas")

    log = _resolve_file(
        root,
        "docs/evidence/tdd.jsonl",
        "EVIDENCE_V1_HASH_MISMATCH",
        "artefacto V1 ausente",
    )
    try:
        lines = log.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeError) as error:
        reject("EVIDENCE_V1_HASH_MISMATCH", "docs/evidence/tdd.jsonl", str(error))
    if len(lines) != value["record_count"]:
        reject(
            "EVIDENCE_V1_RECORD_COUNT",
            "docs/evidence/tdd.jsonl",
            f"esperados 19 registros; encontrados {len(lines)}",
        )

    for entry in [*artifacts, *run_records]:
        relative = str(entry["path"])
        path = _resolve_file(
            root,
            relative,
            "EVIDENCE_V1_HASH_MISMATCH",
            "ruta sellada ausente",
        )
        payload = path.read_bytes()
        actual_digest = hashlib.sha256(payload).hexdigest()
        if len(payload) != entry["bytes"] or actual_digest != entry["sha256"]:
            reject(
                "EVIDENCE_V1_HASH_MISMATCH",
                relative,
                "bytes o SHA-256 difieren del manifest sellado",
            )
    return value, lines


def validate_legacy_records(root: pathlib.Path, lines: list[str]) -> int:
    schema_path = root / "docs/evidence/tdd.schema.json"
    schema = _load_json(
        schema_path,
        "EVIDENCE_V1_RECORD_INVALID",
        "docs/evidence/tdd.schema.json",
    )
    if not isinstance(schema, dict) or schema.get("$schema") != (
        "https://json-schema.org/draft/2020-12/schema"
    ):
        reject(
            "EVIDENCE_V1_RECORD_INVALID",
            "docs/evidence/tdd.schema.json",
            "debe declarar JSON Schema draft 2020-12",
        )

    task_ids: set[str] = set()
    for line_number, line in enumerate(lines, 1):
        where = f"docs/evidence/tdd.jsonl#line={line_number}"
        if not line.strip():
            reject("EVIDENCE_V1_RECORD_INVALID", where, "línea vacía")
        try:
            record = json.loads(line)
            validate_legacy_record(record, line_number, root)
        except (ValueError, json.JSONDecodeError) as error:
            reject("EVIDENCE_V1_RECORD_INVALID", where, str(error))
        task_id = record["task_id"]
        if task_id in task_ids:
            reject("EVIDENCE_V1_RECORD_INVALID", where, f"task_id duplicado: {task_id}")
        task_ids.add(task_id)
    return len(lines)


def validate_v2_schema(root: pathlib.Path) -> None:
    display = "specs/schemas/evidence-record-v2.schema.json"
    schema = _load_json(root / display, "EVIDENCE_V2_SCHEMA_INVALID", display)
    if not isinstance(schema, dict):
        reject("EVIDENCE_V2_SCHEMA_INVALID", display, "el schema debe ser un objeto")
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        reject("EVIDENCE_V2_SCHEMA_INVALID", display, "draft 2020-12 ausente")
    if schema.get("type") != "object" or schema.get("additionalProperties") is not False:
        reject("EVIDENCE_V2_SCHEMA_INVALID", display, "la raíz debe ser un objeto cerrado")
    if set(schema.get("required", [])) != V2_REQUIRED:
        reject("EVIDENCE_V2_SCHEMA_INVALID", display, "required no coincide con EvidenceRecordV2")
    try:
        closed = (
            schema["$defs"]["command"]["additionalProperties"] is False
            and schema["$defs"]["mutation"]["additionalProperties"] is False
            and schema["properties"]["spec_snapshot"]["additionalProperties"] is False
        )
    except (KeyError, TypeError):
        closed = False
    if not closed:
        reject("EVIDENCE_V2_SCHEMA_INVALID", display, "los objetos anidados deben ser cerrados")


def _requirement_ids(root: pathlib.Path) -> set[str]:
    display = "specs/anchors.json"
    anchors = _load_json(root / display, "EVIDENCE_V2_REFERENCE_UNKNOWN", display)
    if not isinstance(anchors, dict) or not isinstance(anchors.get("capabilities"), list):
        reject("EVIDENCE_V2_REFERENCE_UNKNOWN", display, "registro de anchors inválido")
    found: set[str] = set()
    for capability in anchors["capabilities"]:
        if not isinstance(capability, dict) or not isinstance(
            capability.get("requirements"), list
        ):
            reject("EVIDENCE_V2_REFERENCE_UNKNOWN", display, "requisitos de anchors inválidos")
        for requirement in capability["requirements"]:
            if isinstance(requirement, dict) and isinstance(requirement.get("id"), str):
                found.add(requirement["id"])
    return found


def _validate_v2_command(value: object, where: str, mutation: bool = False) -> None:
    expected = {"command", "exit_code", "summary"}
    if mutation:
        expected.add("killed")
    if not isinstance(value, dict):
        reject("EVIDENCE_V2_SCHEMA_INVALID", where, "debe ser un objeto")
    extra = sorted(set(value) - expected)
    if extra:
        reject("EVIDENCE_V2_UNKNOWN_FIELD", where, f"campos desconocidos: {extra}")
    missing = sorted(expected - set(value))
    if missing:
        reject("EVIDENCE_V2_SCHEMA_INVALID", where, f"campos ausentes: {missing}")
    if not isinstance(value["command"], str) or not value["command"].strip():
        reject("EVIDENCE_V2_SCHEMA_INVALID", where, "command debe ser texto no vacío")
    if type(value["exit_code"]) is not int or value["exit_code"] < 0:
        reject("EVIDENCE_V2_SCHEMA_INVALID", where, "exit_code debe ser entero no negativo")
    if not isinstance(value["summary"], str) or not value["summary"].strip():
        reject("EVIDENCE_V2_SCHEMA_INVALID", where, "summary debe ser texto no vacío")
    if mutation and value["killed"] is not True:
        reject("EVIDENCE_V2_SCHEMA_INVALID", where, "killed debe ser true")


def validate_v2_record(
    record: object,
    line_number: int,
    root: pathlib.Path,
    requirements: set[str],
) -> None:
    where = f"docs/evidence/tdd-v2.jsonl#line={line_number}"
    if not isinstance(record, dict):
        reject("EVIDENCE_V2_SCHEMA_INVALID", where, "cada línea debe ser un objeto JSON")
    extra = sorted(set(record) - V2_REQUIRED)
    if extra:
        reject("EVIDENCE_V2_UNKNOWN_FIELD", where, f"campos desconocidos: {extra}")
    missing = sorted(V2_REQUIRED - set(record))
    if missing:
        reject("EVIDENCE_V2_SCHEMA_INVALID", where, f"campos ausentes: {missing}")
    if type(record["schema_version"]) is not int or record["schema_version"] != 2:
        reject("EVIDENCE_V2_SCHEMA_INVALID", where, "schema_version debe ser 2")

    feature_id = record["feature_id"]
    task_id = record["task_id"]
    requirement_ids = record["requirement_ids"]
    mode = record["evidence_mode"]
    if not isinstance(feature_id, str) or not FEATURE_ID.fullmatch(feature_id):
        reject("EVIDENCE_V2_SCHEMA_INVALID", where, "feature_id inválido")
    if not isinstance(task_id, str) or not TASK_ID.fullmatch(task_id):
        reject("EVIDENCE_V2_SCHEMA_INVALID", where, "task_id inválido")
    if (
        not isinstance(requirement_ids, list)
        or not requirement_ids
        or not all(
            isinstance(requirement_id, str)
            and REQUIREMENT_ID.fullmatch(requirement_id)
            for requirement_id in requirement_ids
        )
    ):
        reject("EVIDENCE_V2_SCHEMA_INVALID", where, "requirement_ids inválido")
    if requirement_ids != sorted(requirement_ids) or len(requirement_ids) != len(
        set(requirement_ids)
    ):
        reject("EVIDENCE_V2_SCHEMA_INVALID", where, "requirement_ids debe ser ordenado y único")
    if mode not in {"tdd", "reconstructed_audit"}:
        reject("EVIDENCE_V2_SCHEMA_INVALID", where, "evidence_mode desconocido")

    package = root / "specs" / feature_id
    if not package.is_dir():
        reject(
            "EVIDENCE_V2_REFERENCE_UNKNOWN",
            where,
            f"feature desconocida: {feature_id}",
        )
    tasks_path = package / "tasks.md"
    if not tasks_path.is_file():
        reject("EVIDENCE_V2_REFERENCE_UNKNOWN", where, f"tasks ausente para {feature_id}")
    task_ids = set(
        re.findall(
            r"^- \[[ xX]\] (T[0-9]{3})\b",
            tasks_path.read_text(encoding="utf-8"),
            re.MULTILINE,
        )
    )
    if task_id not in task_ids:
        reject("EVIDENCE_V2_REFERENCE_UNKNOWN", where, f"task desconocida: {task_id}")
    unknown_requirements = sorted(set(requirement_ids) - requirements)
    if unknown_requirements:
        reject(
            "EVIDENCE_V2_REFERENCE_UNKNOWN",
            where,
            f"requisitos desconocidos: {unknown_requirements}",
        )

    snapshot = record["spec_snapshot"]
    snapshot_where = f"{where}.spec_snapshot"
    if not isinstance(snapshot, dict):
        reject("EVIDENCE_V2_SCHEMA_INVALID", snapshot_where, "debe ser un objeto")
    snapshot_extra = sorted(set(snapshot) - {"path", "sha256"})
    if snapshot_extra:
        reject(
            "EVIDENCE_V2_UNKNOWN_FIELD",
            snapshot_where,
            f"campos desconocidos: {snapshot_extra}",
        )
    if set(snapshot) != {"path", "sha256"}:
        reject("EVIDENCE_V2_SCHEMA_INVALID", snapshot_where, "path o sha256 ausente")
    snapshot_path = snapshot["path"]
    snapshot_digest = snapshot["sha256"]
    if (
        not isinstance(snapshot_path, str)
        or not re.fullmatch(r"docs/evidence/specs/[0-9a-f]{64}\.md", snapshot_path)
        or not isinstance(snapshot_digest, str)
        or not SHA256.fullmatch(snapshot_digest)
    ):
        reject("EVIDENCE_V2_SNAPSHOT_HASH", snapshot_where, "snapshot no direccionado por contenido")
    filename_digest = pathlib.PurePosixPath(snapshot_path).stem
    if filename_digest != snapshot_digest:
        reject("EVIDENCE_V2_SNAPSHOT_HASH", snapshot_where, "nombre y sha256 no coinciden")
    snapshot_file = _resolve_file(
        root,
        snapshot_path,
        "EVIDENCE_V2_SNAPSHOT_HASH",
        "snapshot ausente",
    )
    actual_digest = hashlib.sha256(snapshot_file.read_bytes()).hexdigest()
    if actual_digest != snapshot_digest:
        reject("EVIDENCE_V2_SNAPSHOT_HASH", snapshot_path, "bytes y sha256 no coinciden")

    _validate_v2_command(record["red"], f"{where}.red")
    if record["red"]["exit_code"] == 0:
        reject("EVIDENCE_V2_RED_NOT_FAILING", f"{where}.red", "exit_code debe ser distinto de cero")
    _validate_v2_command(record["green"], f"{where}.green")
    if record["green"]["exit_code"] != 0:
        reject("EVIDENCE_V2_SCHEMA_INVALID", f"{where}.green", "exit_code debe ser cero")
    _validate_v2_command(record["mutation"], f"{where}.mutation", mutation=True)
    if mode == "reconstructed_audit" and not record["red"]["summary"].startswith(
        "reconstructed audit:"
    ):
        reject(
            "EVIDENCE_V2_PROVENANCE_INVALID",
            f"{where}.red.summary",
            "debe empezar por 'reconstructed audit:'",
        )
    recorded_at = record["recorded_at"]
    if not isinstance(recorded_at, str) or not UTC_TIMESTAMP.fullmatch(recorded_at):
        reject("EVIDENCE_V2_SCHEMA_INVALID", where, "recorded_at debe ser UTC explícito")


def validate_v2_records(root: pathlib.Path) -> int:
    display = "docs/evidence/tdd-v2.jsonl"
    log = _resolve_file(root, display, "EVIDENCE_V2_LOG_INVALID", "log V2 ausente")
    try:
        lines = log.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeError) as error:
        reject("EVIDENCE_V2_LOG_INVALID", display, str(error))
    if not lines:
        reject("EVIDENCE_V2_LOG_INVALID", display, "debe contener al menos un registro")
    requirements = _requirement_ids(root)
    for line_number, line in enumerate(lines, 1):
        where = f"{display}#line={line_number}"
        if not line.strip():
            reject("EVIDENCE_V2_LOG_INVALID", where, "línea vacía")
        try:
            record = json.loads(line)
        except json.JSONDecodeError as error:
            reject("EVIDENCE_V2_LOG_INVALID", where, f"JSON inválido: {error}")
        validate_v2_record(record, line_number, root, requirements)
    return len(lines)


def validate_repository(root: pathlib.Path) -> tuple[int, int]:
    root = root.resolve()
    _, legacy_lines = validate_v1_manifest(root)
    legacy_count = validate_legacy_records(root, legacy_lines)
    validate_v2_schema(root)
    current_count = validate_v2_records(root)
    return legacy_count, current_count


def main() -> int:
    legacy_count, current_count = validate_repository(ROOT)
    print(
        f"validated {legacy_count} legacy V1 and {current_count} EvidenceRecordV2 record(s)"
    )
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except EvidenceError as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"[EVIDENCE_IO_ERROR] .: {error}", file=sys.stderr)
        sys.exit(1)
