from __future__ import annotations

import gc
import os
import tempfile
import unittest
import weakref
from pathlib import Path

from postproject import OriginIdentity, Production, RevisionContext


LIBRARY_PATH = os.environ.get("POSTPROJECT_LIBRARY")


class ProductionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if LIBRARY_PATH is None:
            raise RuntimeError("POSTPROJECT_LIBRARY must name the native test library")

    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        self.production_path = self.root / "production.pproj"
        self.media_path = self.root / "A001.mov"
        self.media_path.write_bytes(b"Python binding media fixture")

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    def test_create_commit_reopen_and_identity(self) -> None:
        with Production.create(
            self.production_path,
            "Documentary",
            library_path=LIBRARY_PATH,
        ) as production:
            production_id = production.id
            with production.transaction(
                origin=OriginIdentity(
                    "python-test", "1.0", "https://example.com/python-test"
                ),
                message="Import original",
            ) as transaction:
                asset_id = transaction.import_media(self.media_path, "Camera A")
            self.assertTrue(production.contains_asset(asset_id))

        with Production.open(
            self.production_path, library_path=LIBRARY_PATH
        ) as reopened:
            self.assertEqual(reopened.id, production_id)
            self.assertTrue(reopened.contains_asset(asset_id))

    def test_context_exception_and_explicit_rollback_discard_imports(self) -> None:
        with Production.create(
            self.production_path, library_path=LIBRARY_PATH
        ) as production:
            with self.assertRaisesRegex(RuntimeError, "abort"):
                with production.transaction() as transaction:
                    exception_asset = transaction.import_media(self.media_path)
                    raise RuntimeError("abort")
            self.assertFalse(production.contains_asset(exception_asset))

            with production.transaction() as transaction:
                explicit_asset = transaction.import_media(self.media_path)
                transaction.rollback()
            self.assertFalse(production.contains_asset(explicit_asset))

    def test_close_is_idempotent_and_closed_handles_are_rejected(self) -> None:
        production = Production.create(
            self.production_path, library_path=LIBRARY_PATH
        )
        transaction = production.transaction()
        transaction.close()
        transaction.close()
        with self.assertRaisesRegex(RuntimeError, "transaction is closed"):
            transaction.set_revision_context(RevisionContext(message="closed"))

        production.close()
        production.close()
        with self.assertRaisesRegex(RuntimeError, "production is closed"):
            _ = production.id

    def test_finalizer_releases_an_open_transaction(self) -> None:
        with Production.create(
            self.production_path, library_path=LIBRARY_PATH
        ) as production:
            transaction = production.transaction()
            reference = weakref.ref(transaction)
            del transaction
            gc.collect()
            self.assertIsNone(reference())

            with production.transaction() as replacement:
                replacement.rollback()

    def test_embedded_nul_is_rejected_before_native_call(self) -> None:
        with self.assertRaisesRegex(ValueError, "NUL"):
            Production.open("invalid\0path.pproj", library_path=LIBRARY_PATH)


if __name__ == "__main__":
    unittest.main()
