#!/usr/bin/env python3
"""Validate Batuta's red/green/mutation JSONL evidence without third-party deps."""

from __future__ import annotations

import hashlib
import json
import pathlib
import re
import sys


ROOT = pathlib.Path(__file__).resolve().parents[1]
LOG = ROOT / "docs" / "evidence" / "tdd.jsonl"
SCHEMA = ROOT / "docs" / "evidence" / "tdd.schema.json"
TASK_ID = re.compile(r"^K[0-7]\.\d+$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
REQUIRED = {
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


def validate_record(record: object, line_number: int) -> None:
    where = f"line {line_number}"
    if not isinstance(record, dict):
        fail(f"{where} must be a JSON object")
    missing = REQUIRED - set(record)
    if missing:
        fail(f"{where} is missing {sorted(missing)}")
    extra = set(record) - REQUIRED
    if extra:
        fail(f"{where} has unknown fields {sorted(extra)}")
    if not isinstance(record["task_id"], str) or not TASK_ID.fullmatch(record["task_id"]):
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
        path = ROOT / relative
        if not path.is_file():
            fail(f"{where} references missing SPEC {relative}")
    snapshot = record["spec_snapshot"]
    if not isinstance(snapshot, str) or not re.fullmatch(
        r"docs/evidence/specs/[0-9a-f]{64}\.md", snapshot
    ):
        fail(f"{where}.spec_snapshot must be content-addressed")
    snapshot_path = ROOT / snapshot
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


def main() -> int:
    if not SCHEMA.is_file():
        fail(f"missing {SCHEMA.relative_to(ROOT)}")
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    if schema.get("$schema") != "https://json-schema.org/draft/2020-12/schema":
        fail("tdd.schema.json must declare JSON Schema draft 2020-12")
    if not LOG.is_file():
        fail(f"missing {LOG.relative_to(ROOT)}")
    records = 0
    task_ids: set[str] = set()
    for line_number, line in enumerate(LOG.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            fail(f"line {line_number} is blank")
        record = json.loads(line)
        validate_record(record, line_number)
        if record["task_id"] in task_ids:
            fail(f"line {line_number}.task_id is duplicated")
        task_ids.add(record["task_id"])
        records += 1
    if records == 0:
        fail("tdd.jsonl must contain at least one record")
    print(f"validated {records} evidence record(s)")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"TDD evidence invalid: {error}", file=sys.stderr)
        sys.exit(1)
