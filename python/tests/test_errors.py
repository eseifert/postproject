from __future__ import annotations

import unittest

from postproject import (
    AmbiguousResolutionError,
    FingerprintError,
    InternalError,
    IoError,
    MigrationError,
)
from postproject._errors import ERROR_TYPES


class ErrorMappingTests(unittest.TestCase):
    def test_specialized_native_failures_have_public_exception_types(self) -> None:
        self.assertIs(ERROR_TYPES[4], IoError)
        self.assertIs(ERROR_TYPES[6], MigrationError)
        self.assertIs(ERROR_TYPES[8], AmbiguousResolutionError)
        self.assertIs(ERROR_TYPES[9], FingerprintError)
        self.assertIs(ERROR_TYPES[255], InternalError)


if __name__ == "__main__":
    unittest.main()
