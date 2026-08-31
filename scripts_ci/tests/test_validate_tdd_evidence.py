"""Directed mutation tests for the TDD evidence validator."""

import copy
import json
import unittest

from scripts_ci import validate_tdd_evidence


class TddEvidenceValidatorTest(unittest.TestCase):
    def test_record_without_evidence_mode_is_rejected(self) -> None:
        line = validate_tdd_evidence.LOG.read_text(encoding="utf-8").splitlines()[0]
        record = json.loads(line)
        del record["evidence_mode"]

        with self.assertRaisesRegex(ValueError, "evidence_mode"):
            validate_tdd_evidence.validate_record(record, 1)

    def test_snapshot_filename_mutation_is_killed(self) -> None:
        line = validate_tdd_evidence.LOG.read_text(encoding="utf-8").splitlines()[0]
        record = json.loads(line)
        record["spec_sha256"] = "0" * 64

        with self.assertRaisesRegex(ValueError, "immutable snapshot bytes"):
            validate_tdd_evidence.validate_record(record, 1)

    def test_zero_exit_code_in_red_evidence_is_killed(self) -> None:
        line = validate_tdd_evidence.LOG.read_text(encoding="utf-8").splitlines()[0]
        record = json.loads(line)
        mutated = copy.deepcopy(record)
        mutated["evidence_mode"] = "tdd"
        mutated["red"]["exit_code"] = 0

        with self.assertRaisesRegex(ValueError, "red must record a failing command"):
            validate_tdd_evidence.validate_record(mutated, 1)


if __name__ == "__main__":
    unittest.main()
