#include <postproject/postproject.hpp>

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <exception>
#include <filesystem>
#include <fstream>
#include <string>
#include <utility>

int main(int argc, char **argv) {
  if (argc != 2) {
    return 2;
  }

  const std::string path(argv[1]);
  std::remove(path.c_str());

  try {
    if (postproject::abi_version() != 23) {
      return 3;
    }

    auto production = postproject::Production::create(path, "C++ smoke test");
    const auto created_id = production.id();
    const std::string media_path = path + ".media";
    {
      std::ofstream media(media_path, std::ios::binary);
      media << "C++ transaction media";
      if (!media) {
        return 10;
      }
    }
    auto transaction = production.beginTransaction();
    transaction.setRevisionContext(
        {postproject::OriginIdentity{"C++ smoke", std::string("1.0"),
                                     std::nullopt},
         std::string("Import fixture")});
    const auto asset_id = transaction.importMedia(media_path, "C++ asset");
    const postproject::ObjectRef asset_ref{postproject::ObjectKind::asset,
                                           asset_id};
    const postproject::HostObjectBinding host_binding{created_id, asset_ref};
    const auto host_binding_text = host_binding.toString();
    if (host_binding_text.rfind("https://postproject.org/ref/v1/", 0) != 0 ||
        !(postproject::HostObjectBinding::fromString(host_binding_text) ==
          host_binding)) {
      return 21;
    }
    const postproject::ExternalIdentifier external_id{
        "com.example.asset", "asset-42", std::string("primary")};
    transaction.addExternalIdentifier(asset_ref, external_id);
    const auto root_id = transaction.addMediaRoot("fixtures", "Fixture media");
    transaction.commit();
    const auto latest_revision = production.latestRevision();
    const auto revision_page = production.changesSince(0, 1);
    if (!latest_revision.has_value() || latest_revision->sequence != 1 ||
        !latest_revision->origin.has_value() ||
        latest_revision->origin->name != "C++ smoke" ||
        latest_revision->origin->version != std::string("1.0") ||
        latest_revision->message != std::string("Import fixture") ||
        revision_page.size() != 1 ||
        revision_page[0].id != latest_revision->id) {
      return 15;
    }
    const auto revision_events = production.revisionEvents(latest_revision->id);
    if (revision_events.size() != 7 || revision_events[0].position != 0 ||
        !std::holds_alternative<postproject::AssetImportedEvent>(
            revision_events[0].payload) ||
        std::get<postproject::AssetImportedEvent>(revision_events[0].payload)
                .asset_id != asset_id ||
        !std::holds_alternative<postproject::ExternalIdentifierAddedEvent>(
            revision_events[5].payload) ||
        std::get<postproject::ExternalIdentifierAddedEvent>(
            revision_events[5].payload)
                .identifier.value != external_id.value ||
        !std::holds_alternative<postproject::MediaRootAddedEvent>(
            revision_events[6].payload)) {
      return 16;
    }
    if (!production.containsAsset(asset_id)) {
      return 8;
    }
    const auto assets = production.assets();
    if (assets.size() != 1 || assets[0].id != asset_id ||
        assets[0].created_at_unix_micros == 0 ||
        assets[0].display_name != std::string("C++ asset") ||
        assets[0].import_source.has_value()) {
      return 24;
    }
    const auto roots = production.mediaRoots();
    if (roots.size() != 1 || roots[0].id != root_id ||
        roots[0].name != "fixtures" ||
        roots[0].label != std::string("Fixture media") ||
        roots[0].legacy_uri.has_value() || roots[0].priority != 0 ||
        !roots[0].enabled) {
      return 25;
    }
    const auto representations = production.representations(asset_id);
    if (representations.size() != 1 ||
        representations[0].asset_id != asset_id ||
        representations[0].kind != postproject::RepresentationKind::original ||
        representations[0].structure_kind !=
            postproject::ContentStructureKind::single_resource ||
        representations[0].members.size() != 1 ||
        !representations[0].members[0].required ||
        representations[0].members[0].role.has_value() ||
        representations[0].image_sequence.has_value() ||
        representations[0].fingerprints.size() != 1 ||
        representations[0].fingerprints[0].version != 1 ||
        representations[0].fingerprints[0].value.empty() ||
        representations[0].resources.size() != 1 ||
        representations[0].resources[0].id !=
            representations[0].members[0].resource_id ||
        representations[0].resources[0].file_size != UINT64_C(21) ||
        !representations[0].resources[0].modified_at_unix_micros.has_value() ||
        representations[0].resources[0].fingerprints.size() != 1 ||
        representations[0].resources[0].fingerprints[0].version != 1 ||
        representations[0].resources[0].fingerprints[0].value.empty() ||
        representations[0].resources[0].locators.size() != 1 ||
        representations[0].resources[0].locators[0].availability !=
            postproject::LocatorAvailability::online ||
        !representations[0]
             .resources[0]
             .locators[0]
             .last_seen_unix_micros.has_value()) {
      return 17;
    }
    if (production.dependencySet(representations[0].id).has_value() ||
        !production.dependents(asset_ref).empty()) {
      return 30;
    }
    auto observations = production.beginTransaction();
    observations.recordResourceFingerprint(
        representations[0].resources[0].id,
        {"cpp-smoke", 1, {UINT8_C(0x10), UINT8_C(0x20)}});
    observations.recordRepresentationFingerprint(
        representations[0].id,
        {"cpp-smoke-tree", 1, {UINT8_C(0x30), UINT8_C(0x40)}});
    observations.commit();
    const auto observation_revision = production.latestRevision();
    if (!observation_revision.has_value()) {
      return 28;
    }
    const auto observation_events =
        production.revisionEvents(observation_revision->id);
    if (observation_events.size() != 2 ||
        !std::holds_alternative<postproject::ResourceFingerprintObservedEvent>(
            observation_events[0].payload) ||
        !std::holds_alternative<
            postproject::RepresentationFingerprintObservedEvent>(
            observation_events[1].payload)) {
      return 29;
    }
    const auto observed_representations = production.representations(asset_id);
    if (observed_representations[0].fingerprints.size() != 2 ||
        observed_representations[0].resources[0].fingerprints.size() != 2) {
      return 26;
    }
    const auto identifiers = production.externalIdentifiers(asset_ref);
    const auto found =
        production.findByExternalIdentifier("com.example.asset", "asset-42");
    if (identifiers.size() != 1 ||
        identifiers[0].scheme != external_id.scheme ||
        identifiers[0].value != external_id.value ||
        identifiers[0].qualifier != external_id.qualifier || found.size() != 1 ||
        !(found[0] == asset_ref)) {
      return 13;
    }

    auto rolled_back = production.beginTransaction();
    const auto discarded_id = rolled_back.importMedia(media_path);
    rolled_back.rollback();
    if (production.containsAsset(discarded_id)) {
      return 9;
    }

    const std::string moved_media_path = media_path + ".moved";
    std::filesystem::rename(media_path, moved_media_path);
    const auto resolutions = production.resolveAsset(
        asset_id,
        {{"fixtures", std::filesystem::path(path).parent_path().string()}});
    if (resolutions.size() != 1 ||
        resolutions[0].availability !=
            postproject::RepresentationAvailability::online ||
        resolutions[0].resources.size() != 1 ||
        resolutions[0].resources[0].state !=
            postproject::ResourceResolutionState::resolved_exact ||
        resolutions[0].resources[0].candidates.size() != 1 ||
        resolutions[0]
                .resources[0]
                .candidates[0]
                .confidence_basis_points != 10000 ||
        resolutions[0].resources[0].candidates[0].evidence.empty()) {
      return 11;
    }

    const postproject::ActivitySpec activity_spec{
        "org.postproject:ingest",
        100,
        200,
        postproject::ToolIdentity{
            "C++ ingest", std::string("1.0"),
            std::string("https://example.com/tools/ingest")},
        postproject::AgentIdentity{
            std::string("C++ operator"),
            postproject::ExternalIdentifier{"com.example.agent", "operator-1",
                                            std::string("primary")}},
        {},
        {{resolutions[0].representation_id,
          std::string("org.postproject:output.master")}}};
    auto provenance = production.beginTransaction();
    const auto activity_id = provenance.createActivity(activity_spec);
    std::vector<postproject::MetadataInput> metadata_items;
    metadata_items.push_back(postproject::MetadataInput::rational(24000, 1001));
    metadata_items.push_back(
        postproject::MetadataInput::languageString("Interview", "en-US"));
    std::vector<postproject::MetadataFieldInput> metadata_fields;
    metadata_fields.push_back(
        {"values", postproject::MetadataInput::list(metadata_items)});
    const auto metadata =
        postproject::MetadataInput::structure(metadata_fields);
    provenance.addMetadataValue(
        {postproject::ObjectKind::activity, activity_id},
        "com.example.ingest", "details", metadata);
    provenance.commit();

    const auto activities = production.activities();
    const auto producing =
        production.activitiesProducing(resolutions[0].representation_id);
    if (activities.size() != 1 || producing.size() != 1 ||
        activities[0].id != activity_id ||
        activities[0].kind != "org.postproject:ingest" ||
        activities[0].started_at_unix_micros != 100 ||
        activities[0].finished_at_unix_micros != 200 ||
        !activities[0].tool.has_value() ||
        activities[0].tool->name != "C++ ingest" ||
        !activities[0].agent.has_value() ||
        activities[0].agent->name != std::string("C++ operator") ||
        activities[0].outputs.size() != 1 ||
        activities[0].outputs[0].representation_id !=
            resolutions[0].representation_id ||
        activities[0].outputs[0].role !=
            std::string("org.postproject:output.master") ||
        !activities[0].outputs[0].snapshot.has_value() ||
        activities[0].outputs[0].snapshot->revision_sequence == 0 ||
        activities[0].outputs[0].snapshot->fingerprints.empty() ||
        !activities[0]
             .outputs[0]
             .snapshot->fingerprints[0]
             .observed_revision_sequence.has_value() ||
        !production.ancestors(resolutions[0].representation_id).empty()) {
      return 14;
    }
    const auto artifact =
        production.evaluateArtifact(resolutions[0].representation_id);
    const auto reproducibility =
        production.artifactReproducibility(resolutions[0].representation_id);
    if (artifact.state != postproject::ArtifactKnowledgeState::current ||
        artifact.visited_representations != 1 || artifact.truncated ||
        !artifact.reasons.empty() || !reproducibility.reproducible ||
        reproducibility.producing_activity_id != activity_id ||
        reproducibility.activity_kind !=
            std::string("org.postproject:ingest") ||
        !reproducibility.issues.empty()) {
      return 28;
    }
    auto confirmation = production.beginTransaction();
    confirmation.confirmLocator(
        resolutions[0].resources[0].resource_id,
        resolutions[0].resources[0].candidates[0].uri);
    confirmation.setMediaRootEnabled(root_id, false);
    confirmation.retireLocator(
        representations[0].resources[0].locators[0].id);
    confirmation.commit();
    const auto disabled_roots = production.mediaRoots();
    if (disabled_roots.size() != 1 || disabled_roots[0].enabled) {
      return 26;
    }
    auto root_removal = production.beginTransaction();
    root_removal.removeMediaRoot(root_id);
    root_removal.commit();
    if (!production.mediaRoots().empty()) {
      return 27;
    }

    auto moved = std::move(production);
    if (production || !moved) {
      return 4;
    }

    auto reopened = postproject::Production::open(path);
    if (reopened.id() != created_id || !reopened.containsAsset(asset_id)) {
      return 5;
    }
    const auto persisted = reopened.resolveAsset(asset_id);
    if (persisted.size() != 1 ||
        persisted[0].availability !=
            postproject::RepresentationAvailability::online ||
        persisted[0].resources.size() != 1 ||
        persisted[0].resources[0].state !=
            postproject::ResourceResolutionState::online_at_known_locator) {
      return 12;
    }

    const auto sequence_frame_path =
        std::filesystem::path(path).parent_path() / "frame0001.exr";
    {
      std::ofstream frame(sequence_frame_path, std::ios::binary);
      frame << "sequence frame";
      if (!frame) {
        return 22;
      }
    }
    auto additions = reopened.beginTransaction();
    const auto proxy_id = additions.addSingleFileRepresentation(
        asset_id, postproject::RepresentationKind::proxy, moved_media_path);
    const auto sequence_id = additions.addImageSequenceRepresentation(
        asset_id, postproject::RepresentationKind::derived,
        {sequence_frame_path.parent_path().string(), "frame", ".exr", 4, 1,
         1, 1, 24000, 1001, {}});
    const std::vector<postproject::FileResourceInput> ordered_members{
        {moved_media_path, "org.postproject:essence.first", true},
        {sequence_frame_path.string(), "org.postproject:essence.second", true},
    };
    const auto ordered_id = additions.addOrderedPartsRepresentation(
        asset_id, postproject::RepresentationKind::optimized, ordered_members);
    const std::vector<postproject::FileResourceInput> package_members{
        {moved_media_path, "org.postproject:essence", true},
        {sequence_frame_path.string(), "org.postproject:sidecar", false},
    };
    const auto package_id = additions.addPackageRepresentation(
        asset_id, postproject::RepresentationKind::derived, package_members);
    additions.commit();

    const auto added_representations = reopened.representations(asset_id);
    const auto has_representation = [&](const postproject::Uuid &id,
                                        postproject::ContentStructureKind kind) {
      return std::any_of(
          added_representations.begin(), added_representations.end(),
          [&](const postproject::Representation &representation) {
            return representation.id == id &&
                   representation.structure_kind == kind;
          });
    };
    if (added_representations.size() != 5 ||
        !has_representation(
            proxy_id, postproject::ContentStructureKind::single_resource) ||
        !has_representation(
            sequence_id, postproject::ContentStructureKind::image_sequence) ||
        !has_representation(
            ordered_id, postproject::ContentStructureKind::ordered_parts) ||
        !has_representation(package_id,
                            postproject::ContentStructureKind::package)) {
      return 23;
    }

    const postproject::Dependency dependency{
        std::nullopt,
        "org.postproject:reference.character",
        asset_ref,
        resolutions[0].representation_id,
        true,
        "characters/lead.pproj#character/A"};
    auto dependency_update = reopened.beginTransaction();
    dependency_update.recordDependencySet(proxy_id, {dependency});
    dependency_update.commit();
    const auto dependency_revision = reopened.latestRevision();
    if (!dependency_revision.has_value()) {
      return 32;
    }
    const auto dependency_events =
        reopened.revisionEvents(dependency_revision->id);
    const auto dependencies = reopened.dependencySet(proxy_id);
    const auto dependents = reopened.dependents(asset_ref);
    if (!dependencies.has_value() ||
        dependencies->source_representation_id != proxy_id ||
        dependencies->recorded_at_revision == 0 ||
        dependencies->status != postproject::DependencySetStatus::current ||
        dependencies->dependencies.size() != 1 ||
        dependencies->dependencies[0].source_resource_id.has_value() ||
        dependencies->dependencies[0].kind != dependency.kind ||
        !(dependencies->dependencies[0].target == dependency.target) ||
        dependencies->dependencies[0].resolved_representation_id !=
            dependency.resolved_representation_id ||
        !dependencies->dependencies[0].required ||
        dependencies->dependencies[0].authored_reference !=
            dependency.authored_reference ||
        dependents.size() != 1 || dependents[0] != proxy_id ||
        dependency_events.size() != 1 ||
        !std::holds_alternative<postproject::DependencySetRecordedEvent>(
            dependency_events[0].payload) ||
        std::get<postproject::DependencySetRecordedEvent>(
            dependency_events[0].payload)
                .representation_id != proxy_id) {
      return 31;
    }

    auto job_request = reopened.beginTransaction();
    const auto job_id = job_request.requestJob(
        {"org.postproject:generate-proxy", {resolutions[0].representation_id},
         asset_id, postproject::RepresentationKind::proxy, std::nullopt});
    job_request.commit();
    const auto jobs = reopened.jobs();
    if (jobs.size() != 1 || jobs[0].id != job_id ||
        jobs[0].kind != "org.postproject:generate-proxy" ||
        jobs[0].inputs !=
            std::vector<postproject::Uuid>{resolutions[0].representation_id} ||
        jobs[0].output_asset_id != asset_id ||
        jobs[0].output_representation_kind !=
            postproject::RepresentationKind::proxy ||
        jobs[0].target_root.has_value() ||
        jobs[0].state != postproject::JobState::requested ||
        jobs[0].claim.has_value() || jobs[0].completion.has_value() ||
        jobs[0].failure_diagnostic.has_value()) {
      return 33;
    }

    auto claim = reopened.beginTransaction();
    const auto claim_id = claim.claimJob(
        job_id, {"C++ worker", std::string("1.0"), std::nullopt},
        postproject::AgentIdentity{std::string("operator"), std::nullopt}, 10,
        20);
    claim.commit();
    const auto claimed_jobs = reopened.jobs();
    if (claimed_jobs.size() != 1 || !claimed_jobs[0].claim.has_value() ||
        claimed_jobs[0].state != postproject::JobState::claimed ||
        claimed_jobs[0].claim->id != claim_id ||
        claimed_jobs[0].claim->tool.name != "C++ worker" ||
        claimed_jobs[0].claim->tool.version != std::string("1.0") ||
        !claimed_jobs[0].claim->agent.has_value() ||
        claimed_jobs[0].claim->agent->name != std::string("operator") ||
        claimed_jobs[0].claim->expires_at_unix_micros != 20) {
      return 34;
    }

    auto renew = reopened.beginTransaction();
    renew.renewJobClaim(job_id, claim_id, 11, 30);
    renew.commit();
    auto release = reopened.beginTransaction();
    release.releaseJobClaim(job_id, claim_id);
    release.commit();

    auto second_claim = reopened.beginTransaction();
    const auto second_claim_id = second_claim.claimJob(
        job_id, {"C++ worker", std::nullopt, std::nullopt}, std::nullopt, 31,
        40);
    second_claim.commit();
    auto fail = reopened.beginTransaction();
    fail.failJob(job_id, second_claim_id, 32, "encoder exited");
    fail.commit();

    auto second_request = reopened.beginTransaction();
    const auto cancelled_job_id = second_request.requestJob(
        {"org.postproject:generate-thumbnail",
         {resolutions[0].representation_id}, asset_id,
         postproject::RepresentationKind::derived, std::nullopt});
    second_request.commit();
    auto cancel = reopened.beginTransaction();
    cancel.cancelJob(cancelled_job_id);
    cancel.commit();

    const auto final_jobs = reopened.jobs();
    const auto failed_job = std::find_if(
        final_jobs.begin(), final_jobs.end(),
        [&](const postproject::Job &candidate) { return candidate.id == job_id; });
    const auto cancelled_job = std::find_if(
        final_jobs.begin(), final_jobs.end(), [&](const postproject::Job &candidate) {
          return candidate.id == cancelled_job_id;
        });
    if (final_jobs.size() != 2 || failed_job == final_jobs.end() ||
        failed_job->state != postproject::JobState::failed ||
        failed_job->claim.has_value() ||
        failed_job->failure_diagnostic != std::string("encoder exited") ||
        cancelled_job == final_jobs.end() ||
        cancelled_job->state != postproject::JobState::cancelled) {
      return 35;
    }

    auto completion_request = reopened.beginTransaction();
    const auto completed_job_id = completion_request.requestJob(
        {"org.postproject:generate-proxy",
         {resolutions[0].representation_id}, asset_id,
         postproject::RepresentationKind::proxy, std::nullopt});
    completion_request.commit();
    auto completion_claim = reopened.beginTransaction();
    const auto completion_claim_id = completion_claim.claimJob(
        completed_job_id, {"C++ worker", std::nullopt, std::nullopt},
        std::nullopt, 41, 50);
    completion_claim.commit();
    auto completion = reopened.beginTransaction();
    const auto completed_representation_id =
        completion.addSingleFileRepresentation(
            asset_id, postproject::RepresentationKind::proxy,
            moved_media_path);
    const auto completion_activity_id = completion.createActivity(
        {"org.postproject:transcode",
         std::nullopt,
         std::nullopt,
         postproject::ToolIdentity{"C++ worker", std::nullopt, std::nullopt},
         std::nullopt,
         {{resolutions[0].representation_id,
           std::string("org.postproject:input.primary-video")}},
         {{completed_representation_id,
           std::string("org.postproject:output.proxy")}}});
    completion.completeJob(completed_job_id, completion_claim_id, 42,
                           completed_representation_id,
                           completion_activity_id);
    completion.commit();

    const auto completed_jobs = reopened.jobs();
    const auto completed_job = std::find_if(
        completed_jobs.begin(), completed_jobs.end(),
        [&](const postproject::Job &candidate) {
          return candidate.id == completed_job_id;
        });
    const auto completion_producing =
        reopened.activitiesProducing(completed_representation_id);
    if (completed_jobs.size() != 3 || completed_job == completed_jobs.end() ||
        completed_job->state != postproject::JobState::succeeded ||
        !completed_job->completion.has_value() ||
        completed_job->completion->activity_id != completion_activity_id ||
        completed_job->completion->representation_id !=
            completed_representation_id ||
        completion_producing.size() != 1 ||
        completion_producing[0].id != completion_activity_id ||
        !completion_producing[0].outputs[0].snapshot.has_value()) {
      return 36;
    }

    const auto regeneration_plans = reopened.planRegeneration(
        {resolutions[0].representation_id, resolutions[0].representation_id});
    if (regeneration_plans.size() != 1 ||
        regeneration_plans[0].artifact_representation_id !=
            resolutions[0].representation_id ||
        regeneration_plans[0].job.kind != "org.postproject:ingest" ||
        !regeneration_plans[0].job.inputs.empty() ||
        regeneration_plans[0].job.output_asset_id != asset_id ||
        regeneration_plans[0].job.output_representation_kind !=
            postproject::RepresentationKind::original ||
        regeneration_plans[0].job.state !=
            postproject::JobState::requested ||
        regeneration_plans[0].parameters.size() != 1 ||
        regeneration_plans[0].parameters[0].vocabulary !=
            "com.example.ingest" ||
        regeneration_plans[0].parameters[0].property != "details" ||
        reopened.jobs().size() != 3) {
      return 37;
    }

    try {
      static_cast<void>(postproject::Production::open(path + ".missing"));
      return 6;
    } catch (const postproject::Error &error) {
      if (error.code() == postproject::ErrorCode::ok ||
          std::string(error.what()).empty()) {
        return 7;
      }
    }
  } catch (const std::exception &error) {
    std::fprintf(stderr, "%s\n", error.what());
    return 1;
  }

  return 0;
}
