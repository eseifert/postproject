from __future__ import annotations

import gc
import os
import tempfile
import unittest
import weakref
from pathlib import Path
from uuid import UUID

from postproject import (
    AssetImportedEvent,
    ExternalIdentifier,
    ExternalIdentifierAddedEvent,
    ExternalIdentifierRemovedEvent,
    InvalidArgumentError,
    LocatorAddedEvent,
    NotFoundError,
    OriginIdentity,
    Production,
    RepresentationAddedEvent,
    RepresentationResourceAddedEvent,
    ResourceAddedEvent,
    RevisionContext,
    RevisionId,
)


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
            self.assertIn(asset_id, production.assets)

        with Production.open(
            self.production_path, library_path=LIBRARY_PATH
        ) as reopened:
            self.assertEqual(reopened.id, production_id)
            self.assertIn(asset_id, reopened.assets)

    def test_context_exception_and_explicit_rollback_discard_imports(self) -> None:
        with Production.create(
            self.production_path, library_path=LIBRARY_PATH
        ) as production:
            with self.assertRaisesRegex(RuntimeError, "abort"):
                with production.transaction() as transaction:
                    exception_asset = transaction.import_media(self.media_path)
                    raise RuntimeError("abort")
            self.assertNotIn(exception_asset, production.assets)

            with production.transaction() as transaction:
                explicit_asset = transaction.import_media(self.media_path)
                transaction.rollback()
            self.assertNotIn(explicit_asset, production.assets)

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

    def test_revision_summaries_are_copied_and_paginated(self) -> None:
        with Production.create(
            self.production_path, library_path=LIBRARY_PATH
        ) as production:
            self.assertIsNone(production.latest_revision)
            with production.transaction(
                origin=OriginIdentity("python-test", "1.0"),
                message="First import",
            ) as transaction:
                transaction.import_media(self.media_path)

            first = production.latest_revision
            self.assertIsNotNone(first)
            assert first is not None
            self.assertEqual(first.sequence, 1)
            self.assertEqual(first.message, "First import")
            self.assertEqual(first.origin, OriginIdentity("python-test", "1.0"))

            with production.transaction() as transaction:
                transaction.import_media(self.media_path, "Second")

            page = production.changes_since(0, 1)
            self.assertEqual(page, (first,))
            second_page = production.changes_since(page[-1].sequence, 1)
            self.assertEqual(len(second_page), 1)
            self.assertEqual(second_page[0].sequence, 2)
            self.assertIsNone(second_page[0].origin)
            self.assertIsNone(second_page[0].message)

            with self.assertRaises(InvalidArgumentError):
                production.changes_since(0, 0)

    def test_revision_events_are_typed_ordered_and_copied(self) -> None:
        with Production.create(
            self.production_path, library_path=LIBRARY_PATH
        ) as production:
            with production.transaction() as transaction:
                asset_id = transaction.import_media(self.media_path)

            revision = production.latest_revision
            assert revision is not None
            events = production.revision_events[revision.id]

            self.assertEqual([event.position for event in events], list(range(5)))
            imported, representation, resource, membership, locator = (
                event.payload for event in events
            )
            assert isinstance(imported, AssetImportedEvent)
            assert isinstance(representation, RepresentationAddedEvent)
            assert isinstance(resource, ResourceAddedEvent)
            assert isinstance(membership, RepresentationResourceAddedEvent)
            assert isinstance(locator, LocatorAddedEvent)
            self.assertEqual(imported.asset_id, asset_id)
            self.assertEqual(representation.asset_id, asset_id)
            self.assertEqual(membership.representation_id, representation.representation_id)
            self.assertEqual(membership.resource_id, resource.resource_id)
            self.assertEqual(membership.structural_position, 0)
            self.assertEqual(locator.resource_id, resource.resource_id)

            missing = RevisionId(UUID("00000000-0000-0000-0000-000000000001"))
            with self.assertRaises(NotFoundError):
                _ = production.revision_events[missing]

    def test_external_identifiers_roundtrip_lookup_and_remove(self) -> None:
        camera_id = ExternalIdentifier(
            "com.example.camera", "A001-C023", "primary"
        )
        umid = ExternalIdentifier(
            "urn:smpte:umid", "060A2B340101010501010D4313000000"
        )
        with Production.create(
            self.production_path, library_path=LIBRARY_PATH
        ) as production:
            with production.transaction() as transaction:
                asset_id = transaction.import_media(self.media_path)

            with production.transaction() as transaction:
                transaction.add_external_identifier(asset_id, camera_id)
                transaction.add_external_identifier(asset_id, umid)

            self.assertEqual(
                set(production.external_identifiers[asset_id]), {camera_id, umid}
            )
            self.assertEqual(
                production.objects_by_external_identifier[
                    camera_id.scheme, camera_id.value
                ],
                (asset_id,),
            )
            revision = production.latest_revision
            assert revision is not None
            added = production.revision_events[revision.id]
            self.assertTrue(
                all(
                    isinstance(event.payload, ExternalIdentifierAddedEvent)
                    for event in added
                )
            )

            with production.transaction() as transaction:
                transaction.remove_external_identifier(asset_id, camera_id)

            self.assertEqual(production.external_identifiers[asset_id], (umid,))
            self.assertEqual(
                production.objects_by_external_identifier[
                    camera_id.scheme, camera_id.value
                ],
                (),
            )
            revision = production.latest_revision
            assert revision is not None
            removed = production.revision_events[revision.id]
            self.assertEqual(len(removed), 1)
            payload = removed[0].payload
            assert isinstance(payload, ExternalIdentifierRemovedEvent)
            self.assertEqual(payload.target, asset_id)
            self.assertEqual(payload.identifier, camera_id)

    def test_external_identifier_nul_is_rejected_before_native_call(self) -> None:
        with Production.create(
            self.production_path, library_path=LIBRARY_PATH
        ) as production:
            with self.assertRaisesRegex(ValueError, "NUL"):
                with production.transaction() as transaction:
                    transaction.add_external_identifier(
                        production.id, ExternalIdentifier("invalid\0scheme", "value")
                    )


if __name__ == "__main__":
    unittest.main()
