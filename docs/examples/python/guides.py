"""Run every Python listing included in the PostProject integrator guides.

Each ``[name]`` ... ``[/name]`` region is included verbatim by the
documentation build, so keep regions self-contained and readable. Usage::

    POSTPROJECT_LIBRARY=/path/to/libpostproject.so python guides.py WORK_DIRECTORY

The work directory is prepared by ``prepare-workdir.cmake``.
"""

from __future__ import annotations

import sys
from pathlib import Path

from postproject import (
    ActivityEdge,
    ActivitySpec,
    AssetId,
    Dependency,
    ExternalIdentifier,
    ImageSequenceInput,
    JobRequest,
    JobState,
    MetadataLanguageString,
    MetadataProperty,
    OriginIdentity,
    Production,
    RepresentationId,
    RepresentationKind,
    RepresentationResolution,
    RevisionEvent,
    ToolIdentity,
)


# [create-production]
def create_production(path: Path, media: Path) -> tuple[Production, AssetId]:
    production = Production.create(path, "Documentary")
    with production.transaction(
        origin=OriginIdentity("com.example.editor", "0.4.0"),
        message="Import camera original",
    ) as transaction:
        asset_id = transaction.import_media(media, display_name="Camera A")

    print(f"representations: {len(production.representations[asset_id])}")
    return production, asset_id


# [/create-production]


# [external-identifiers]
def tag_camera_serial(production: Production, asset_id: AssetId) -> None:
    identifier = ExternalIdentifier("com.example.camera.serial", "A-0007")
    with production.transaction() as transaction:
        transaction.add_external_identifier(asset_id, identifier)

    attached = production.external_identifiers[asset_id]
    matches = production.objects_by_external_identifier[
        identifier.scheme, identifier.value
    ]
    assert attached == (identifier,)
    assert matches == (asset_id,)


# [/external-identifiers]


# [metadata]
def add_title(production: Production, asset_id: AssetId) -> None:
    title = MetadataProperty(
        "https://iptc.org/std/videometadatahub/recommendation/iptc-vmhub-1.7-schema.json",
        "title",
    )
    with production.transaction() as transaction:
        transaction.add_metadata(
            asset_id, title, MetadataLanguageString("Interview", "en-US")
        )

    for assertion in production.metadata[asset_id]:
        print(f"{assertion.property.property}: {assertion.value}")
    assert len(production.metadata_by_property[title]) == 1


# [/metadata]


# [media-root]
def add_rushes_root(production: Production) -> None:
    with production.transaction() as transaction:
        transaction.add_media_root("rushes", "Camera originals")


# [/media-root]


# [resolve-asset]
def resolve_asset(
    production: Production, asset_id: AssetId, rushes_directory: Path
) -> tuple[RepresentationResolution, ...]:
    resolutions = production.resolve(asset_id, {"rushes": rushes_directory})
    for representation in resolutions:
        print(f"availability: {representation.availability.name}")
        for resource in representation.resources:
            for candidate in resource.candidates:
                print(
                    f"candidate: {candidate.uri} "
                    f"({candidate.confidence_basis_points}/10000)"
                )
    return resolutions


# [/resolve-asset]


# [confirm-locator]
def confirm_unique_candidates(
    production: Production, resolutions: tuple[RepresentationResolution, ...]
) -> None:
    with production.transaction() as transaction:
        for representation in resolutions:
            for resource in representation.resources:
                # Several candidates need a person to choose; never pick one here.
                if len(resource.candidates) == 1:
                    transaction.confirm_locator(
                        resource.resource_id, resource.candidates[0].uri
                    )


# [/confirm-locator]


# [image-sequence]
def add_render_sequence(
    production: Production, asset_id: AssetId, directory: Path
) -> RepresentationId:
    with production.transaction() as transaction:
        sequence_id = transaction.add_image_sequence_representation(
            asset_id,
            RepresentationKind.DERIVED,
            ImageSequenceInput(
                directory=str(directory),
                prefix="shot010.",
                suffix=".exr",
                padding=4,
                start=1001,
                end=1004,
                step=1,
                rate_numerator=24000,
                rate_denominator=1001,
                missing_frames=(1003,),
            ),
        )

    stored = next(
        item for item in production.representations[asset_id] if item.id == sequence_id
    )
    sequence = stored.image_sequence
    assert sequence is not None
    print(
        f"{sequence.prefix}#{sequence.suffix} frames {sequence.start}-{sequence.end}, "
        f"{len(sequence.missing_frames)} known missing"
    )
    return sequence_id


# [/image-sequence]


# [provenance]
def record_render(
    production: Production,
    source_id: RepresentationId,
    render_id: RepresentationId,
) -> None:
    with production.transaction() as transaction:
        activity_id = transaction.create_activity(
            ActivitySpec(
                "org.postproject:render",
                inputs=(ActivityEdge(source_id, "org.postproject:primary"),),
                outputs=(ActivityEdge(render_id),),
                tool=ToolIdentity(
                    "Example Renderer", "2.1", "https://example.com/renderer"
                ),
            )
        )

    (producer,) = production.activities_producing[render_id]
    assert producer.id == activity_id
    assert production.activities_consuming[source_id] == (producer,)
    assert production.provenance_ancestors[render_id] == (source_id,)
    assert production.provenance_descendants[source_id] == (render_id,)


# [/provenance]


# [artifact-knowledge]
def inspect_artifact(production: Production, artifact_id: RepresentationId) -> None:
    evaluation = production.evaluate_artifact(
        artifact_id, max_depth=64, max_representations=1000
    )
    print(f"artifact state: {evaluation.state.name}")
    for reason in evaluation.reasons:
        print(f"reason: {reason.kind.name}")

    reproducibility = production.artifact_reproducibility(artifact_id)
    print(
        f"reproducible: {reproducibility.reproducible}, "
        f"missing conditions: {len(reproducibility.issues)}"
    )


# [/artifact-knowledge]


# [dependency-queries]
def record_and_query_dependencies(
    production: Production,
    source_id: RepresentationId,
    target_asset_id: AssetId,
    resolved_id: RepresentationId,
) -> None:
    dependency = Dependency(
        kind="org.example:character-reference",
        target=target_asset_id,
        authored_reference="characters/lead.usd",
        resolved_representation_id=resolved_id,
    )
    with production.transaction() as transaction:
        transaction.record_dependency_set(source_id, (dependency,))

    dependencies = production.dependencies(
        source_id, max_depth=4, max_representations=1000, limit=100
    )
    for match in dependencies.items:
        print(f"dependency {match.target} at depth {match.depth}")
    assert not dependencies.traversal_truncated

    dependents = production.dependents(
        target_asset_id, max_depth=4, max_representations=1000, limit=100
    )
    assert dependents.items[0].target == source_id


# [/dependency-queries]


# [job-query-pages]
def request_and_page_jobs(
    production: Production,
    input_id: RepresentationId,
    output_asset_id: AssetId,
) -> None:
    request = JobRequest(
        "org.example:generate-proxy",
        (input_id,),
        output_asset_id,
        RepresentationKind.PROXY,
    )
    with production.transaction() as transaction:
        transaction.request_job(request)
        transaction.request_job(request)

    cursor = None
    count = 0
    while True:
        page = production.jobs(
            limit=1,
            cursor=cursor,
            state=JobState.REQUESTED,
            kind="org.example:generate-proxy",
        )
        count += len(page.items)
        cursor = page.next_cursor
        if cursor is None:
            break
    assert count == 2


# [/job-query-pages]


def handle_event(event: RevisionEvent) -> None:
    print(f"event {event.position}: {type(event.payload).__name__}")


# [revision-feed]
def process_changes(production: Production, cursor: int) -> int:
    limit = 100
    while True:
        page = production.changes_since(cursor, limit)
        for revision in page:
            for event in production.revision_events[revision.id]:
                # Dispatch with isinstance on event.payload.
                handle_event(event)
            # Persist the cursor only after the whole revision is processed.
            cursor = revision.sequence
        if len(page) < limit:
            return cursor


# [/revision-feed]


# [host-binding]
def bind_representation(
    production: Production, representation_id: RepresentationId
) -> str:
    stored = production.host_bindings[representation_id]

    binding = production.host_bindings.parse(stored)
    assert binding.production_id == production.id
    assert binding.object == representation_id
    return stored


# [/host-binding]


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: guides.py WORK_DIRECTORY")
    work = Path(sys.argv[1])
    media = work / "rushes" / "A001.mov"
    moved = work / "moved"

    production, asset_id = create_production(work / "production.pproj", media)
    with production:
        original_id = production.representations[asset_id][0].id
        tag_camera_serial(production, asset_id)
        add_title(production, asset_id)

        add_rushes_root(production)
        media.rename(moved / "A001.mov")
        resolutions = resolve_asset(production, asset_id, moved)
        assert len(resolutions[0].resources[0].candidates) == 1
        confirm_unique_candidates(production, resolutions)

        sequence_id = add_render_sequence(
            production, asset_id, work / "renders" / "shot010"
        )
        record_render(production, original_id, sequence_id)
        inspect_artifact(production, sequence_id)
        record_and_query_dependencies(production, sequence_id, asset_id, original_id)
        request_and_page_jobs(production, original_id, asset_id)

        cursor = process_changes(production, 0)
        latest = production.latest_revision
        assert latest is not None and cursor == latest.sequence
        print(f"binding: {bind_representation(production, sequence_id)}")


if __name__ == "__main__":
    main()
