// Runs every C++ listing included in the PostProject integrator guides.
//
// Each "[name]" ... "[/name]" region is included verbatim by the documentation
// build, so keep regions self-contained and readable. Usage:
//   postproject-cpp-guides WORK_DIRECTORY
// The work directory is prepared by prepare-workdir.cmake.
#include <postproject/postproject.hpp>

#include <cstdint>
#include <cstdio>
#include <iostream>
#include <stdexcept>
#include <string>
#include <utility>
#include <variant>
#include <vector>

namespace {

void require(bool condition, const char *message) {
  if (!condition) {
    throw std::runtime_error(message);
  }
}

// [create-production]
std::pair<postproject::Production, postproject::Uuid>
create_production(const std::string &path, const std::string &media) {
  auto production = postproject::Production::create(path, "Documentary");

  auto transaction = production.beginTransaction();
  transaction.setRevisionContext(
      {postproject::OriginIdentity{"com.example.editor", "0.4.0", std::nullopt},
       "Import camera original"});
  const auto asset_id = transaction.importMedia(media, "Camera A");
  transaction.commit();

  std::cout << "representations: "
            << production.representations(asset_id).size() << '\n';
  return {std::move(production), asset_id};
}
// [/create-production]

// [external-identifiers]
void tag_camera_serial(postproject::Production &production,
                       const postproject::Uuid &asset_id) {
  const postproject::ObjectRef target{postproject::ObjectKind::asset, asset_id};
  const postproject::ExternalIdentifier identifier{
      "com.example.camera.serial", "A-0007", std::nullopt};

  auto transaction = production.beginTransaction();
  transaction.addExternalIdentifier(target, identifier);
  transaction.commit();

  const auto attached = production.externalIdentifiers(target);
  const auto matches =
      production.findByExternalIdentifier(identifier.scheme, identifier.value);
  require(attached.size() == 1 && matches == std::vector{target},
          "identifier lookup");
}
// [/external-identifiers]

// [metadata]
void add_title(postproject::Production &production,
               const postproject::Uuid &asset_id) {
  const postproject::ObjectRef target{postproject::ObjectKind::asset, asset_id};

  auto transaction = production.beginTransaction();
  transaction.addMetadataValue(
      target,
      "https://iptc.org/std/videometadatahub/recommendation/"
      "iptc-vmhub-1.7-schema.json",
      "title", postproject::MetadataInput::languageString("Interview", "en-US"));
  transaction.commit();
  // The wrapper does not read metadata yet; use pp_production_metadata().
}
// [/metadata]

// [media-root]
void add_rushes_root(postproject::Production &production) {
  auto transaction = production.beginTransaction();
  transaction.addMediaRoot("rushes", "Camera originals");
  transaction.commit();
}
// [/media-root]

// [resolve-asset]
std::vector<postproject::RepresentationResolution>
resolve_asset(const postproject::Production &production,
              const postproject::Uuid &asset_id,
              const std::string &rushes_directory) {
  const auto resolutions =
      production.resolveAsset(asset_id, {{"rushes", rushes_directory}});
  for (const auto &representation : resolutions) {
    std::cout << "availability: "
              << static_cast<std::uint32_t>(representation.availability)
              << '\n';
    for (const auto &resource : representation.resources) {
      for (const auto &candidate : resource.candidates) {
        std::cout << "candidate: " << candidate.uri << " ("
                  << candidate.confidence_basis_points << "/10000)\n";
      }
    }
  }
  return resolutions;
}
// [/resolve-asset]

// [confirm-locator]
void confirm_unique_candidates(
    postproject::Production &production,
    const std::vector<postproject::RepresentationResolution> &resolutions) {
  auto transaction = production.beginTransaction();
  for (const auto &representation : resolutions) {
    for (const auto &resource : representation.resources) {
      // Several candidates need a person to choose; never pick one here.
      if (resource.candidates.size() == 1) {
        transaction.confirmLocator(resource.resource_id,
                                   resource.candidates.front().uri);
      }
    }
  }
  transaction.commit();
}
// [/confirm-locator]

// [image-sequence]
postproject::Uuid add_render_sequence(postproject::Production &production,
                                      const postproject::Uuid &asset_id,
                                      const std::string &directory) {
  postproject::ImageSequenceInput sequence{};
  sequence.directory = directory;
  sequence.prefix = "shot010.";
  sequence.suffix = ".exr";
  sequence.padding = 4;
  sequence.start = 1001;
  sequence.end = 1004;
  sequence.step = 1;
  sequence.rate_numerator = 24000;
  sequence.rate_denominator = 1001;
  sequence.missing_frames = {1003};

  auto transaction = production.beginTransaction();
  const auto sequence_id = transaction.addImageSequenceRepresentation(
      asset_id, postproject::RepresentationKind::derived, sequence);
  transaction.commit();

  for (const auto &representation : production.representations(asset_id)) {
    if (representation.id == sequence_id && representation.image_sequence) {
      const auto &stored = *representation.image_sequence;
      std::cout << stored.prefix << '#' << stored.suffix << " frames "
                << stored.start << '-' << stored.end << ", "
                << stored.missing_frames.size() << " known missing\n";
    }
  }
  return sequence_id;
}
// [/image-sequence]

// [provenance]
void record_render(postproject::Production &production,
                   const postproject::Uuid &source_id,
                   const postproject::Uuid &render_id) {
  postproject::ActivitySpec activity{};
  activity.kind = "org.postproject:render";
  activity.inputs = {{source_id, "org.postproject:primary"}};
  activity.outputs = {{render_id, std::nullopt}};
  activity.tool = postproject::ToolIdentity{
      "Example Renderer", "2.1", "https://example.com/renderer"};

  auto transaction = production.beginTransaction();
  const auto activity_id = transaction.createActivity(activity);
  transaction.commit();

  const auto producers = production.activitiesProducing(render_id);
  require(producers.size() == 1 && producers.front().id == activity_id,
          "producing activity");
  require(production.ancestors(render_id) == std::vector{source_id},
          "provenance ancestors");
  require(production.descendants(source_id) == std::vector{render_id},
          "provenance descendants");
}
// [/provenance]

void handle_event(const postproject::RevisionEvent &event) {
  std::cout << "event " << event.position << ": alternative "
            << event.payload.index() << '\n';
}

// [revision-feed]
std::uint64_t process_changes(const postproject::Production &production,
                              std::uint64_t cursor) {
  constexpr std::uint32_t limit = 100;
  for (;;) {
    const auto page = production.changesSince(cursor, limit);
    for (const auto &revision : page) {
      for (const auto &event : production.revisionEvents(revision.id)) {
        // Dispatch with std::visit on event.payload.
        handle_event(event);
      }
      // Persist the cursor only after the whole revision is processed.
      cursor = revision.sequence;
    }
    if (page.size() < limit) {
      return cursor;
    }
  }
}
// [/revision-feed]

// [host-binding]
std::string bind_representation(const postproject::Production &production,
                                const postproject::Uuid &representation_id) {
  const postproject::HostObjectBinding binding{
      production.id(), {postproject::ObjectKind::representation,
                        representation_id}};
  const std::string stored = binding.toString();

  const auto reopened = postproject::HostObjectBinding::fromString(stored);
  require(reopened == binding, "binding round trip");
  return stored;
}
// [/host-binding]

} // namespace

int main(int argc, char **argv) {
  if (argc != 2) {
    std::cerr << "usage: postproject-cpp-guides WORK_DIRECTORY\n";
    return 2;
  }
  const std::string work = argv[1];
  const std::string media = work + "/rushes/A001.mov";
  const std::string moved = work + "/moved";

  try {
    auto [production, asset_id] =
        create_production(work + "/production.pproj", media);
    const auto original_id = production.representations(asset_id).front().id;
    tag_camera_serial(production, asset_id);
    add_title(production, asset_id);

    add_rushes_root(production);
    require(std::rename(media.c_str(), (moved + "/A001.mov").c_str()) == 0,
            "move media");
    const auto resolutions = resolve_asset(production, asset_id, moved);
    require(resolutions.front().resources.front().candidates.size() == 1,
            "unique candidate");
    confirm_unique_candidates(production, resolutions);

    const auto sequence_id = add_render_sequence(
        production, asset_id, work + "/renders/shot010");
    record_render(production, original_id, sequence_id);

    const auto cursor = process_changes(production, 0);
    require(cursor == production.latestRevision()->sequence, "feed cursor");
    std::cout << "binding: " << bind_representation(production, sequence_id)
              << '\n';
  } catch (const std::exception &error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
