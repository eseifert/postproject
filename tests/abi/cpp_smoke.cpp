#include <postproject/postproject.hpp>

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
    if (postproject::abi_version() != 10) {
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
    static_cast<void>(transaction.addMediaRoot(
        std::filesystem::path(path).parent_path().string(), "fixtures"));
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
        !representations[0].fingerprints.empty() ||
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
    const auto resolutions = production.resolveAsset(asset_id);
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
        !production.ancestors(resolutions[0].representation_id).empty()) {
      return 14;
    }
    auto confirmation = production.beginTransaction();
    confirmation.confirmLocator(
        resolutions[0].resources[0].resource_id,
        resolutions[0].resources[0].candidates[0].uri);
    confirmation.commit();

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
