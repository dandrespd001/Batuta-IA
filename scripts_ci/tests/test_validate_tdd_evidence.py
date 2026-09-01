"""Mutation tests for preservation of V1 and the EvidenceRecordV2 contract."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import shutil
import tempfile
import unittest

from scripts_ci import validate_tdd_evidence


FIXTURES = Path(__file__).parent / "fixtures" / "tdd_evidence"
VALID_RECORD = FIXTURES / "valid.json"
SNAPSHOT_SHA256 = "5af921cd3337311540d79e50b86bff536797cba47b3c4ec3f459b27aace7937e"


class TddEvidenceValidatorTest(unittest.TestCase):
    def _copy(self, source: Path, destination: Path) -> None:
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)

    def _materialize(
        self, record_fixture: Path = VALID_RECORD
    ) -> tuple[tempfile.TemporaryDirectory[str], Path]:
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        source_root = validate_tdd_evidence.ROOT

        baseline_source = source_root / "docs/evidence/v1-baseline.json"
        baseline = json.loads(baseline_source.read_text(encoding="utf-8"))
        self._copy(baseline_source, root / "docs/evidence/v1-baseline.json")
        sealed = [*baseline["artifacts"], *baseline["run_records"]]
        for entry in sealed:
            relative = Path(entry["path"])
            self._copy(source_root / relative, root / relative)

        for line in (source_root / "docs/evidence/tdd.jsonl").read_text(
            encoding="utf-8"
        ).splitlines():
            for relative_text in json.loads(line)["spec_paths"]:
                relative = Path(relative_text)
                self._copy(source_root / relative, root / relative)

        support = (
            "specs/anchors.json",
            "specs/001-adopt-spec-anchoring/tasks.md",
            "specs/schemas/evidence-record-v2.schema.json",
        )
        for relative_text in support:
            relative = Path(relative_text)
            self._copy(source_root / relative, root / relative)

        snapshot = FIXTURES / "snapshots" / f"{SNAPSHOT_SHA256}.md"
        self._copy(snapshot, root / "docs/evidence/specs" / snapshot.name)
        self._copy(record_fixture, root / "docs/evidence/tdd-v2.jsonl")
        return temporary, root

    def _assert_invalid(self, fixture: str, code: str) -> str:
        temporary, root = self._materialize(FIXTURES / "invalid" / fixture)
        self.addCleanup(temporary.cleanup)
        with self.assertRaises(validate_tdd_evidence.EvidenceError) as context:
            validate_tdd_evidence.validate_repository(root)
        diagnostic = str(context.exception)
        self.assertIn(f"[{code}]", diagnostic)
        self.assertNotIn(str(root), diagnostic)
        return diagnostic

    def test_real_repository_preserves_seven_v1_paths_and_nineteen_records(self) -> None:
        baseline = json.loads(
            validate_tdd_evidence.V1_BASELINE.read_text(encoding="utf-8")
        )
        self.assertEqual(6, len(baseline["artifacts"]))
        self.assertEqual(1, len(baseline["run_records"]))
        self.assertEqual(19, baseline["record_count"])
        self.assertEqual(19, len(validate_tdd_evidence.V1_LOG.read_text().splitlines()))

        legacy, current = validate_tdd_evidence.validate_repository(
            validate_tdd_evidence.ROOT
        )
        self.assertEqual((19, 1), (legacy, current))

    def test_each_of_the_seven_sealed_v1_paths_rejects_a_byte_change(self) -> None:
        baseline = json.loads(
            validate_tdd_evidence.V1_BASELINE.read_text(encoding="utf-8")
        )
        for entry in [*baseline["artifacts"], *baseline["run_records"]]:
            with self.subTest(path=entry["path"]):
                temporary, root = self._materialize()
                try:
                    target = root / entry["path"]
                    payload = bytearray(target.read_bytes())
                    payload[0] ^= 1
                    target.write_bytes(payload)
                    with self.assertRaisesRegex(
                        validate_tdd_evidence.EvidenceError,
                        r"\[EVIDENCE_V1_HASH_MISMATCH\]",
                    ):
                        validate_tdd_evidence.validate_repository(root)
                finally:
                    temporary.cleanup()

    def test_v1_record_count_is_independent_from_the_file_hash(self) -> None:
        temporary, root = self._materialize()
        self.addCleanup(temporary.cleanup)
        log = root / "docs/evidence/tdd.jsonl"
        first = log.read_text(encoding="utf-8").splitlines()[0]
        log.write_text(
            log.read_text(encoding="utf-8") + first + "\n", encoding="utf-8"
        )

        manifest_path = root / "docs/evidence/v1-baseline.json"
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        entry = next(
            item
            for item in manifest["artifacts"]
            if item["path"].endswith("tdd.jsonl")
        )
        entry["bytes"] = len(log.read_bytes())
        entry["sha256"] = hashlib.sha256(log.read_bytes()).hexdigest()
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

        with self.assertRaisesRegex(
            validate_tdd_evidence.EvidenceError,
            r"\[EVIDENCE_V1_RECORD_COUNT\]",
        ):
            validate_tdd_evidence.validate_repository(root)

    def test_evidence_v2_schema_is_closed_draft_2020_12(self) -> None:
        schema = json.loads(
            validate_tdd_evidence.V2_SCHEMA.read_text(encoding="utf-8")
        )
        expected = {
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
        self.assertEqual(
            "https://json-schema.org/draft/2020-12/schema", schema["$schema"]
        )
        self.assertFalse(schema["additionalProperties"])
        self.assertEqual(expected, set(schema["required"]))
        self.assertFalse(schema["$defs"]["command"]["additionalProperties"])
        self.assertFalse(schema["$defs"]["mutation"]["additionalProperties"])
        self.assertFalse(
            schema["properties"]["spec_snapshot"]["additionalProperties"]
        )

    def test_v2_invalid_fixtures_emit_their_stable_diagnostics(self) -> None:
        cases = {
            "unknown_field.json": "EVIDENCE_V2_UNKNOWN_FIELD",
            "unknown_nested_field.json": "EVIDENCE_V2_UNKNOWN_FIELD",
            "unknown_feature.json": "EVIDENCE_V2_REFERENCE_UNKNOWN",
            "unknown_task.json": "EVIDENCE_V2_REFERENCE_UNKNOWN",
            "unknown_requirement.json": "EVIDENCE_V2_REFERENCE_UNKNOWN",
            "snapshot_hash.json": "EVIDENCE_V2_SNAPSHOT_HASH",
            "snapshot_filename.json": "EVIDENCE_V2_SNAPSHOT_HASH",
            "red_not_failing.json": "EVIDENCE_V2_RED_NOT_FAILING",
            "reconstructed_prefix.json": "EVIDENCE_V2_PROVENANCE_INVALID",
        }
        for fixture, code in cases.items():
            with self.subTest(fixture=fixture):
                first = self._assert_invalid(fixture, code)
                second = self._assert_invalid(fixture, code)
                self.assertEqual(first, second)

    def test_changed_snapshot_bytes_are_rejected(self) -> None:
        temporary, root = self._materialize()
        self.addCleanup(temporary.cleanup)
        snapshot = root / "docs/evidence/specs" / f"{SNAPSHOT_SHA256}.md"
        snapshot.write_bytes(snapshot.read_bytes() + b"changed")
        with self.assertRaisesRegex(
            validate_tdd_evidence.EvidenceError,
            r"\[EVIDENCE_V2_SNAPSHOT_HASH\]",
        ):
            validate_tdd_evidence.validate_repository(root)

    def test_record_without_evidence_mode_is_rejected(self) -> None:
        record = json.loads(VALID_RECORD.read_text(encoding="utf-8"))
        del record["evidence_mode"]
        with self.assertRaisesRegex(
            validate_tdd_evidence.EvidenceError,
            r"\[EVIDENCE_V2_SCHEMA_INVALID\]",
        ):
            validate_tdd_evidence.validate_v2_record(
                record,
                line_number=1,
                root=validate_tdd_evidence.ROOT,
                requirements={"REQ-CONTRACTS-001"},
            )


if __name__ == "__main__":
    unittest.main()
