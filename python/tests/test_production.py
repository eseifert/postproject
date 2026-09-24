from __future__ import annotations

import gc
import os
import tempfile
import unittest
import weakref
from pathlib import Path
from uuid import UUID

from postproject import (
    ActivityCreatedEvent,
    ActivityEdge,
    ActivityInputAddedEvent,
    ActivityOutputAddedEvent,
    ActivitySpec,
    AgentIdentity,
    AssetImportedEvent,
    AvailabilityIssueKind,
    ContentStructureKind,
    EvidenceKind,
    ExternalIdentifier,
    ExternalIdentifierAddedEvent,
    ExternalIdentifierRemovedEvent,
    FileResourceInput,
    Fingerprint,
    HostObjectBinding,
    ImageSequenceInput,
    InvalidArgumentError,
    LocatorAddedEvent,
    LocatorAvailability,
    LocatorRetiredEvent,
    MediaRootEnabledChangedEvent,
    MediaRootRemovedEvent,
    MetadataAddedOrReplacedEvent,
    MetadataAssertion,
    MetadataBool,
    MetadataBytes,
    MetadataDecimal,
    MetadataI64,
    MetadataLanguageString,
    MetadataList,
    MetadataProperty,
    MetadataRational,
    MetadataReference,
    MetadataRemovedEvent,
    MetadataString,
    MetadataStruct,
    MetadataStructField,
    MetadataTimestamp,
    MetadataU64,
    MetadataUri,
    NotFoundError,
    OriginIdentity,
    Production,
    RepresentationAddedEvent,
    RepresentationAvailability,
    RepresentationKind,
    RepresentationResourceAddedEvent,
    ResourceAddedEvent,
    ResourceResolutionState,
    RevisionContext,
    RevisionId,
    ToolIdentity,
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
        self.second_media_path = self.root / "A002.mov"
        self.second_media_path.write_bytes(b"Second Python binding media fixture")

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
            assets = tuple(production.assets)
            self.assertEqual(len(assets), 1)
            self.assertEqual(assets[0].id, asset_id)
            self.assertGreater(assets[0].created_at_unix_micros, 0)
            self.assertEqual(assets[0].display_name, "Camera A")
            self.assertIsNone(assets[0].import_source)

        with Production.open(
            self.production_path, library_path=LIBRARY_PATH
        ) as reopened:
            self.assertEqual(reopened.id, production_id)
            self.assertIn(asset_id, reopened.assets)

    def test_representations_are_typed_keyed_and_copied(self) -> None:
        with Production.create(
            self.production_path, library_path=LIBRARY_PATH
        ) as production:
            with production.transaction() as transaction:
                asset_id = transaction.import_media(self.media_path)
            representations = production.representations[asset_id]

        self.assertEqual(len(representations), 1)
        representation = representations[0]
        self.assertEqual(representation.asset_id, asset_id)
        self.assertEqual(representation.kind, RepresentationKind.ORIGINAL)
        self.assertEqual(
            representation.structure_kind, ContentStructureKind.SINGLE_RESOURCE
        )
        self.assertIsNone(representation.image_sequence)
        self.assertEqual(len(representation.fingerprints), 1)
        self.assertEqual(
            representation.fingerprints[0].algorithm, "pp-blake3-representation"
        )
        self.assertEqual(representation.fingerprints[0].version, 1)
        self.assertTrue(representation.fingerprints[0].value)
        self.assertEqual(len(representation.members), 1)
        self.assertTrue(representation.members[0].required)
        self.assertIsNone(representation.members[0].role)

        self.assertEqual(len(representation.resources), 1)
        resource = representation.resources[0]
        self.assertEqual(resource.id, representation.members[0].resource_id)
        self.assertEqual(resource.file_size, len(self.media_path.read_bytes()))
        self.assertIsNotNone(resource.modified_at_unix_micros)
        self.assertEqual(len(resource.fingerprints), 1)
        self.assertEqual(resource.fingerprints[0].version, 1)
        self.assertTrue(resource.fingerprints[0].value)
        self.assertEqual(len(resource.locators), 1)
        self.assertEqual(resource.locators[0].availability, LocatorAvailability.ONLINE)
        self.assertIsNotNone(resource.locators[0].last_seen_unix_micros)

    def test_fingerprint_observations_are_explicit_and_idempotent(self) -> None:
        with Production.create(
            self.production_path, library_path=LIBRARY_PATH
        ) as production:
            with production.transaction() as transaction:
                asset_id = transaction.import_media(self.media_path)
            representation = production.representations[asset_id][0]
            resource_id = representation.resources[0].id
            resource_fingerprint = Fingerprint("python-test", 1, b"resource")
            representation_fingerprint = Fingerprint(
                "python-test-tree", 1, b"representation"
            )
            with production.transaction() as transaction:
                transaction.record_resource_fingerprint(
                    resource_id, resource_fingerprint
                )
                transaction.record_representation_fingerprint(
                    representation.id, representation_fingerprint
                )

            observed = production.representations[asset_id][0]
            self.assertIn(resource_fingerprint, observed.resources[0].fingerprints)
            self.assertIn(representation_fingerprint, observed.fingerprints)
            revision = production.latest_revision
            assert revision is not None

            with production.transaction() as transaction:
                transaction.record_resource_fingerprint(
                    resource_id, resource_fingerprint
                )
            self.assertEqual(production.latest_revision, revision)

    def test_media_roots_and_locators_have_a_complete_lifecycle(self) -> None:
        with Production.create(
            self.production_path, library_path=LIBRARY_PATH
        ) as production:
            with production.transaction() as transaction:
                asset_id = transaction.import_media(self.media_path)
                root_id = transaction.add_media_root("media", "Media", 4)

            roots = production.media_roots
            self.assertEqual(len(roots), 1)
            self.assertEqual(roots[0].id, root_id)
            self.assertEqual(roots[0].name, "media")
            self.assertEqual(roots[0].label, "Media")
            self.assertIsNone(roots[0].legacy_uri)
            self.assertEqual(roots[0].priority, 4)
            self.assertTrue(roots[0].enabled)
            locator_id = (
                production.representations[asset_id][0].resources[0].locators[0].id
            )

            with production.transaction() as transaction:
                transaction.set_media_root_enabled(root_id, False)
                transaction.retire_locator(locator_id)

            self.assertFalse(production.media_roots[0].enabled)
            self.assertEqual(
                production.representations[asset_id][0].resources[0].locators, ()
            )
            revision = production.latest_revision
            self.assertIsNotNone(revision)
            assert revision is not None
            events = production.revision_events[revision.id]
            self.assertIsInstance(events[0].payload, MediaRootEnabledChangedEvent)
            self.assertIsInstance(events[1].payload, LocatorRetiredEvent)

            with production.transaction() as transaction:
                transaction.remove_media_root(root_id)

            self.assertEqual(production.media_roots, ())
            revision = production.latest_revision
            self.assertIsNotNone(revision)
            assert revision is not None
            events = production.revision_events[revision.id]
            self.assertEqual(len(events), 1)
            self.assertIsInstance(events[0].payload, MediaRootRemovedEvent)

    def test_additional_and_compound_representations_roundtrip(self) -> None:
        sequence_frame = self.root / "frame0001.exr"
        sequence_frame.write_bytes(b"sequence frame")
        sidecar = self.root / "clip.xml"
        sidecar.write_text("<metadata />")
        with Production.create(
            self.production_path, library_path=LIBRARY_PATH
        ) as production:
            with production.transaction() as transaction:
                asset_id = transaction.import_media(self.media_path)

            with production.transaction() as transaction:
                proxy_id = transaction.add_single_file_representation(
                    asset_id, RepresentationKind.PROXY, self.second_media_path
                )
                sequence_id = transaction.add_image_sequence_representation(
                    asset_id,
                    RepresentationKind.DERIVED,
                    ImageSequenceInput(
                        str(self.root), "frame", ".exr", 4, 1, 1, 1, 24_000, 1_001
                    ),
                )
                ordered_id = transaction.add_ordered_parts_representation(
                    asset_id,
                    RepresentationKind.OPTIMIZED,
                    (
                        FileResourceInput(
                            str(self.media_path),
                            "org.postproject:essence.first",
                        ),
                        FileResourceInput(
                            str(self.second_media_path),
                            "org.postproject:essence.second",
                        ),
                    ),
                )
                package_id = transaction.add_package_representation(
                    asset_id,
                    RepresentationKind.DERIVED,
                    (
                        FileResourceInput(
                            str(self.media_path), "org.postproject:essence"
                        ),
                        FileResourceInput(
                            str(sidecar), "org.postproject:sidecar", False
                        ),
                    ),
                )

            representations = {
                representation.id: representation
                for representation in production.representations[asset_id]
            }
            self.assertEqual(len(representations), 5)
            self.assertEqual(
                representations[proxy_id].structure_kind,
                ContentStructureKind.SINGLE_RESOURCE,
            )
            sequence = representations[sequence_id]
            self.assertEqual(
                sequence.structure_kind, ContentStructureKind.IMAGE_SEQUENCE
            )
            assert sequence.image_sequence is not None
            self.assertEqual(sequence.image_sequence.rate_numerator, 24_000)
            self.assertEqual(sequence.image_sequence.rate_denominator, 1_001)
            self.assertEqual(
                representations[ordered_id].structure_kind,
                ContentStructureKind.ORDERED_PARTS,
            )
            package = representations[package_id]
            self.assertEqual(package.structure_kind, ContentStructureKind.PACKAGE)
            self.assertEqual(
                tuple(member.required for member in package.members), (True, False)
            )

    def test_host_bindings_are_keyed_and_round_trip_through_native_abi(self) -> None:
        with Production.create(
            self.production_path, library_path=LIBRARY_PATH
        ) as production:
            with production.transaction() as transaction:
                asset_id = transaction.import_media(self.media_path)

            encoded = production.host_bindings[asset_id]
            self.assertTrue(encoded.startswith("https://postproject.org/ref/v1/"))
            self.assertEqual(
                production.host_bindings.parse(encoded),
                HostObjectBinding(production.id, asset_id),
            )
            with self.assertRaises(InvalidArgumentError):
                production.host_bindings.parse("postproject:v1:obsolete")

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
        production = Production.create(self.production_path, library_path=LIBRARY_PATH)
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
            self.assertEqual(
                membership.representation_id, representation.representation_id
            )
            self.assertEqual(membership.resource_id, resource.resource_id)
            self.assertEqual(membership.structural_position, 0)
            self.assertEqual(locator.resource_id, resource.resource_id)

            missing = RevisionId(UUID("00000000-0000-0000-0000-000000000001"))
            with self.assertRaises(NotFoundError):
                _ = production.revision_events[missing]

    def test_external_identifiers_roundtrip_lookup_and_remove(self) -> None:
        camera_id = ExternalIdentifier("com.example.camera", "A001-C023", "primary")
        umid = ExternalIdentifier("urn:smpte:umid", "060A2B340101010501010D4313000000")
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

    def test_resolution_results_are_typed_and_keyed_by_asset(self) -> None:
        with Production.create(
            self.production_path, library_path=LIBRARY_PATH
        ) as production:
            with production.transaction() as transaction:
                asset_id = transaction.import_media(self.media_path)

            resolutions = production.resolutions[asset_id]
            self.assertEqual(len(resolutions), 1)
            representation = resolutions[0]
            self.assertEqual(
                representation.availability, RepresentationAvailability.ONLINE
            )
            self.assertEqual(representation.issues, ())
            self.assertEqual(len(representation.resources), 1)
            resource = representation.resources[0]
            self.assertEqual(
                resource.state,
                ResourceResolutionState.ONLINE_AT_KNOWN_LOCATOR,
            )
            self.assertEqual(resource.evidence, ())
            self.assertEqual(len(resource.candidates), 1)
            candidate = resource.candidates[0]
            self.assertEqual(candidate.uri, self.media_path.resolve().as_uri())
            self.assertEqual(candidate.confidence_basis_points, 10_000)
            self.assertEqual(
                tuple(item.kind for item in candidate.evidence),
                (EvidenceKind.KNOWN_LOCATOR_AVAILABLE,),
            )

    def test_ambiguous_resolution_requires_explicit_confirmation(self) -> None:
        candidates = self.root / "candidates"
        candidates.mkdir()
        (candidates / "a.mov").write_bytes(b"Python binding media fixture")
        (candidates / "b.mov").write_bytes(b"Python binding media fixture")

        with Production.create(
            self.production_path, library_path=LIBRARY_PATH
        ) as production:
            with production.transaction() as transaction:
                asset_id = transaction.import_media(self.media_path)
                transaction.add_media_root("relocated", "Relocated")
            self.media_path.unlink()

            resolution = production.resolve(asset_id, {"relocated": candidates})[0]
            self.assertEqual(
                resolution.availability, RepresentationAvailability.AMBIGUOUS
            )
            self.assertEqual(len(resolution.resources), 1)
            resource = resolution.resources[0]
            self.assertEqual(resource.state, ResourceResolutionState.AMBIGUOUS)
            self.assertEqual(len(resource.candidates), 2)
            self.assertEqual(
                tuple(candidate.uri for candidate in resource.candidates),
                tuple(
                    path.resolve().as_uri()
                    for path in (candidates / "a.mov", candidates / "b.mov")
                ),
            )
            self.assertEqual(len(resolution.issues), 1)
            self.assertEqual(
                resolution.issues[0].kind,
                AvailabilityIssueKind.AMBIGUOUS_RESOURCE,
            )

            with production.transaction() as transaction:
                transaction.confirm_locator(
                    resource.resource_id, resource.candidates[0].uri
                )

            confirmed = production.resolutions[asset_id][0]
            self.assertEqual(confirmed.availability, RepresentationAvailability.ONLINE)
            self.assertEqual(
                confirmed.resources[0].state,
                ResourceResolutionState.ONLINE_AT_KNOWN_LOCATOR,
            )

    def test_external_identifier_nul_is_rejected_before_native_call(self) -> None:
        with (
            Production.create(
                self.production_path, library_path=LIBRARY_PATH
            ) as production,
            self.assertRaisesRegex(ValueError, "NUL"),
        ):
            with production.transaction() as transaction:
                transaction.add_external_identifier(
                    production.id, ExternalIdentifier("invalid\0scheme", "value")
                )

    def test_text_metadata_is_typed_repeatable_searchable_and_removable(self) -> None:
        title = MetadataProperty("https://example.com/metadata", "title")
        plain = MetadataString("Interview")
        localized = MetadataLanguageString("Gespräch", "de")
        with Production.create(
            self.production_path, library_path=LIBRARY_PATH
        ) as production:
            with production.transaction() as transaction:
                asset_id = transaction.import_media(self.media_path)

            with production.transaction() as transaction:
                transaction.add_metadata(asset_id, title, plain)
                transaction.add_metadata(asset_id, title, localized)

            expected = (
                MetadataAssertion(asset_id, title, plain),
                MetadataAssertion(asset_id, title, localized),
            )
            self.assertEqual(production.metadata[asset_id], expected)
            self.assertEqual(production.metadata_by_property[title], expected)
            revision = production.latest_revision
            assert revision is not None
            added = production.revision_events[revision.id]
            self.assertTrue(
                all(
                    isinstance(event.payload, MetadataAddedOrReplacedEvent)
                    for event in added
                )
            )

            with production.transaction() as transaction:
                transaction.remove_metadata_property(asset_id, title)

            self.assertEqual(production.metadata[asset_id], ())
            self.assertEqual(production.metadata_by_property[title], ())
            revision = production.latest_revision
            assert revision is not None
            removed = production.revision_events[revision.id]
            self.assertEqual(len(removed), 1)
            payload = removed[0].payload
            assert isinstance(payload, MetadataRemovedEvent)
            self.assertEqual(payload.target, asset_id)
            self.assertEqual(payload.property, title)

    def test_recursive_typed_metadata_write_roundtrips(self) -> None:
        property = MetadataProperty("https://example.com/metadata", "technical")
        with Production.create(
            self.production_path, library_path=LIBRARY_PATH
        ) as production:
            with production.transaction() as transaction:
                asset_id = transaction.import_media(self.media_path)

            value = MetadataStruct(
                (
                    MetadataStructField("signed", MetadataI64(-42)),
                    MetadataStructField("unsigned", MetadataU64(42)),
                    MetadataStructField("decimal", MetadataDecimal(-12345, 2)),
                    MetadataStructField("enabled", MetadataBool(True)),
                    MetadataStructField(
                        "captured", MetadataTimestamp(1_700_000_000_123_456)
                    ),
                    MetadataStructField(
                        "source", MetadataUri("https://example.com/source")
                    ),
                    MetadataStructField("payload", MetadataBytes(b"\x00\xff")),
                    MetadataStructField("rate", MetadataRational(24_000, 1_001)),
                    MetadataStructField("asset", MetadataReference(asset_id)),
                    MetadataStructField(
                        "labels",
                        MetadataList(
                            (
                                MetadataString("interview"),
                                MetadataLanguageString("Gespräch", "de"),
                            )
                        ),
                    ),
                )
            )
            with production.transaction() as transaction:
                transaction.add_metadata(asset_id, property, value)

            self.assertEqual(
                production.metadata[asset_id],
                (MetadataAssertion(asset_id, property, value),),
            )

    def test_provenance_activity_roundtrips_and_supports_graph_queries(self) -> None:
        with Production.create(
            self.production_path, library_path=LIBRARY_PATH
        ) as production:
            with production.transaction() as transaction:
                transaction.import_media(self.media_path)
            revision = production.latest_revision
            assert revision is not None
            source_event = next(
                event.payload
                for event in production.revision_events[revision.id]
                if isinstance(event.payload, RepresentationAddedEvent)
            )

            with production.transaction() as transaction:
                transaction.import_media(self.second_media_path)
            revision = production.latest_revision
            assert revision is not None
            output_event = next(
                event.payload
                for event in production.revision_events[revision.id]
                if isinstance(event.payload, RepresentationAddedEvent)
            )

            input_edge = ActivityEdge(
                source_event.representation_id, "org.postproject:primary"
            )
            output_edge = ActivityEdge(
                output_event.representation_id, "org.postproject:proxy"
            )
            spec = ActivitySpec(
                "org.postproject:transcode",
                outputs=(output_edge,),
                inputs=(input_edge,),
                started_at_unix_micros=100,
                finished_at_unix_micros=200,
                tool=ToolIdentity("FFmpeg", "8.0", "https://ffmpeg.org/"),
                agent=AgentIdentity(
                    "automation",
                    ExternalIdentifier("com.example.worker", "worker-1"),
                ),
            )
            with production.transaction() as transaction:
                activity_id = transaction.create_activity(spec)

            expected = production.activities[0]
            self.assertEqual(expected.id, activity_id)
            self.assertEqual(expected.kind, spec.kind)
            self.assertEqual(
                expected.inputs[0].representation_id, input_edge.representation_id
            )
            self.assertEqual(
                expected.outputs[0].representation_id, output_edge.representation_id
            )
            input_snapshot = expected.inputs[0].snapshot
            output_snapshot = expected.outputs[0].snapshot
            assert input_snapshot is not None
            assert output_snapshot is not None
            self.assertEqual(input_snapshot.revision_sequence, 3)
            self.assertEqual(output_snapshot.revision_sequence, 3)
            self.assertEqual(
                input_snapshot.fingerprints[0].observed_revision_sequence, 1
            )
            self.assertEqual(
                output_snapshot.fingerprints[0].observed_revision_sequence, 2
            )
            self.assertEqual(production.activities, (expected,))
            self.assertEqual(
                production.activities_consuming[source_event.representation_id],
                (expected,),
            )
            self.assertEqual(
                production.activities_producing[output_event.representation_id],
                (expected,),
            )
            self.assertEqual(
                production.provenance_ancestors[output_event.representation_id],
                (source_event.representation_id,),
            )
            self.assertEqual(
                production.provenance_descendants[source_event.representation_id],
                (output_event.representation_id,),
            )

            revision = production.latest_revision
            assert revision is not None
            payloads = tuple(
                event.payload for event in production.revision_events[revision.id]
            )
            self.assertEqual(len(payloads), 3)
            self.assertIsInstance(payloads[0], ActivityCreatedEvent)
            self.assertIsInstance(payloads[1], ActivityInputAddedEvent)
            self.assertIsInstance(payloads[2], ActivityOutputAddedEvent)


if __name__ == "__main__":
    unittest.main()
