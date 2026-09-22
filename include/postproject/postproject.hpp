#ifndef POSTPROJECT_POSTPROJECT_HPP
#define POSTPROJECT_POSTPROJECT_HPP

#include <postproject/postproject.h>

#include <array>
#include <cstdint>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>
#include <variant>
#include <vector>

namespace postproject {

enum class ErrorCode : std::uint32_t {
  ok = PP_OK,
  invalid_argument = PP_ERROR_INVALID_ARGUMENT,
  not_found = PP_ERROR_NOT_FOUND,
  already_exists = PP_ERROR_ALREADY_EXISTS,
  io = PP_ERROR_IO,
  storage = PP_ERROR_STORAGE,
  migration = PP_ERROR_MIGRATION,
  conflict = PP_ERROR_CONFLICT,
  ambiguous_resolution = PP_ERROR_AMBIGUOUS_RESOLUTION,
  fingerprint = PP_ERROR_FINGERPRINT,
  unsupported = PP_ERROR_UNSUPPORTED,
  internal = PP_ERROR_INTERNAL,
};

class Error final : public std::runtime_error {
public:
  Error(ErrorCode code, std::string message)
      : std::runtime_error(std::move(message)), code_(code) {}

  [[nodiscard]] ErrorCode code() const noexcept { return code_; }

private:
  ErrorCode code_;
};

class Uuid final {
public:
  using Bytes = std::array<std::uint8_t, 16>;

  constexpr explicit Uuid(Bytes bytes) noexcept : bytes_(bytes) {}

  [[nodiscard]] constexpr const Bytes &bytes() const noexcept { return bytes_; }

  friend constexpr bool operator==(const Uuid &left,
                                   const Uuid &right) noexcept {
    for (std::size_t index = 0; index < left.bytes_.size(); ++index) {
      if (left.bytes_[index] != right.bytes_[index]) {
        return false;
      }
    }
    return true;
  }

  friend constexpr bool operator!=(const Uuid &left,
                                   const Uuid &right) noexcept {
    return !(left == right);
  }

private:
  Bytes bytes_;
};

enum class ObjectKind : std::uint32_t {
  production = PP_OBJECT_PRODUCTION,
  asset = PP_OBJECT_ASSET,
  representation = PP_OBJECT_REPRESENTATION,
  resource = PP_OBJECT_RESOURCE,
  activity = PP_OBJECT_ACTIVITY,
};

struct ObjectRef final {
  ObjectKind kind;
  Uuid id;

  friend constexpr bool operator==(const ObjectRef &left,
                                   const ObjectRef &right) noexcept {
    return left.kind == right.kind && left.id == right.id;
  }
};

struct HostObjectBinding final {
  Uuid production_id;
  ObjectRef object;

  [[nodiscard]] std::string toString() const;
  [[nodiscard]] static HostObjectBinding fromString(std::string_view value);

  friend constexpr bool operator==(const HostObjectBinding &left,
                                   const HostObjectBinding &right) noexcept {
    return left.production_id == right.production_id &&
           left.object == right.object;
  }
};

struct ExternalIdentifier final {
  std::string scheme;
  std::string value;
  std::optional<std::string> qualifier;
};

struct ActivityEdge final {
  Uuid representation_id;
  std::optional<std::string> role;
};

struct ToolIdentity final {
  std::string name;
  std::optional<std::string> version;
  std::optional<std::string> uri;
};

struct AgentIdentity final {
  std::optional<std::string> name;
  std::optional<ExternalIdentifier> identifier;
};

struct Activity final {
  Uuid id;
  std::string kind;
  std::optional<std::int64_t> started_at_unix_micros;
  std::optional<std::int64_t> finished_at_unix_micros;
  std::optional<ToolIdentity> tool;
  std::optional<AgentIdentity> agent;
  std::vector<ActivityEdge> inputs;
  std::vector<ActivityEdge> outputs;
};

struct ActivitySpec final {
  std::string kind;
  std::optional<std::int64_t> started_at_unix_micros;
  std::optional<std::int64_t> finished_at_unix_micros;
  std::optional<ToolIdentity> tool;
  std::optional<AgentIdentity> agent;
  std::vector<ActivityEdge> inputs;
  std::vector<ActivityEdge> outputs;
};

struct OriginIdentity final {
  std::string name;
  std::optional<std::string> version;
  std::optional<std::string> uri;
};

struct RevisionContext final {
  std::optional<OriginIdentity> origin;
  std::optional<std::string> message;
};

struct Revision final {
  Uuid id;
  std::uint64_t sequence;
  Uuid transaction_id;
  std::int64_t committed_at_unix_micros;
  std::optional<OriginIdentity> origin;
  std::optional<std::string> message;
};

struct AssetImportedEvent final {
  Uuid asset_id;
};

struct RepresentationAddedEvent final {
  Uuid asset_id;
  Uuid representation_id;
};

struct ResourceAddedEvent final {
  Uuid resource_id;
};

struct RepresentationResourceAddedEvent final {
  Uuid representation_id;
  Uuid resource_id;
  std::uint32_t structural_position;
};

struct LocatorAddedEvent final {
  Uuid resource_id;
  Uuid locator_id;
};

struct MediaRootAddedEvent final {
  Uuid media_root_id;
};

struct ExternalIdentifierAddedEvent final {
  ObjectRef target;
  ExternalIdentifier identifier;
};

struct ExternalIdentifierRemovedEvent final {
  ObjectRef target;
  ExternalIdentifier identifier;
};

struct MetadataAddedOrReplacedEvent final {
  ObjectRef target;
  std::string vocabulary;
  std::string property;
};

struct MetadataRemovedEvent final {
  ObjectRef target;
  std::string vocabulary;
  std::string property;
};

struct ActivityCreatedEvent final {
  Uuid activity_id;
  std::string kind;
};

struct ActivityInputAddedEvent final {
  Uuid activity_id;
  Uuid representation_id;
  std::optional<std::string> role;
};

struct ActivityOutputAddedEvent final {
  Uuid activity_id;
  Uuid representation_id;
  std::optional<std::string> role;
};

using RevisionEventPayload =
    std::variant<AssetImportedEvent, RepresentationAddedEvent,
                 ResourceAddedEvent, RepresentationResourceAddedEvent,
                 LocatorAddedEvent, MediaRootAddedEvent,
                 ExternalIdentifierAddedEvent,
                 ExternalIdentifierRemovedEvent,
                 MetadataAddedOrReplacedEvent, MetadataRemovedEvent,
                 ActivityCreatedEvent, ActivityInputAddedEvent,
                 ActivityOutputAddedEvent>;

struct RevisionEvent final {
  std::uint32_t position;
  RevisionEventPayload payload;
};

enum class RepresentationKind : std::uint32_t {
  original = PP_REPRESENTATION_ORIGINAL,
  proxy = PP_REPRESENTATION_PROXY,
  optimized = PP_REPRESENTATION_OPTIMIZED,
  derived = PP_REPRESENTATION_DERIVED,
};

enum class ContentStructureKind : std::uint32_t {
  single_resource = PP_CONTENT_SINGLE_RESOURCE,
  image_sequence = PP_CONTENT_IMAGE_SEQUENCE,
  ordered_parts = PP_CONTENT_ORDERED_PARTS,
  package = PP_CONTENT_PACKAGE,
};

enum class LocatorAvailability : std::uint32_t {
  unknown = PP_LOCATOR_UNKNOWN,
  online = PP_LOCATOR_ONLINE,
  offline = PP_LOCATOR_OFFLINE,
};

struct Fingerprint final {
  std::string algorithm;
  std::uint16_t version;
  std::vector<std::uint8_t> value;
};

struct RepresentationMember final {
  Uuid resource_id;
  std::optional<std::string> role;
  bool required;
};

struct ImageSequenceDescriptor final {
  std::string prefix;
  std::string suffix;
  std::uint8_t padding;
  std::int64_t start;
  std::int64_t end;
  std::uint32_t step;
  std::uint32_t rate_numerator;
  std::uint32_t rate_denominator;
  std::vector<std::int64_t> missing_frames;
};

struct Locator final {
  Uuid id;
  std::string uri;
  LocatorAvailability availability;
  std::optional<std::int64_t> last_seen_unix_micros;
};

struct Resource final {
  Uuid id;
  std::optional<std::uint64_t> file_size;
  std::optional<std::int64_t> modified_at_unix_micros;
  std::vector<Fingerprint> fingerprints;
  std::vector<Locator> locators;
};

struct Representation final {
  Uuid id;
  Uuid asset_id;
  RepresentationKind kind;
  ContentStructureKind structure_kind;
  std::vector<RepresentationMember> members;
  std::optional<ImageSequenceDescriptor> image_sequence;
  std::vector<Fingerprint> fingerprints;
  std::vector<Resource> resources;
};

enum class RepresentationAvailability : std::uint32_t {
  online = PP_AVAILABILITY_ONLINE,
  partial = PP_AVAILABILITY_PARTIAL,
  offline = PP_AVAILABILITY_OFFLINE,
  ambiguous = PP_AVAILABILITY_AMBIGUOUS,
  error = PP_AVAILABILITY_ERROR,
};

enum class ResourceResolutionState : std::uint32_t {
  online_at_known_locator = PP_RESOURCE_ONLINE_AT_KNOWN_LOCATOR,
  resolved_exact = PP_RESOURCE_RESOLVED_EXACT,
  resolved_probable = PP_RESOURCE_RESOLVED_PROBABLE,
  offline = PP_RESOURCE_OFFLINE,
  ambiguous = PP_RESOURCE_AMBIGUOUS,
  error = PP_RESOURCE_RESOLUTION_ERROR,
};

enum class AvailabilityIssueKind : std::uint32_t {
  offline_resource = PP_AVAILABILITY_ISSUE_OFFLINE_RESOURCE,
  ambiguous_resource = PP_AVAILABILITY_ISSUE_AMBIGUOUS_RESOURCE,
  resource_error = PP_AVAILABILITY_ISSUE_RESOURCE_ERROR,
  missing_frames = PP_AVAILABILITY_ISSUE_MISSING_FRAMES,
};

enum class EvidenceKind : std::uint32_t {
  known_locator_available = PP_EVIDENCE_KNOWN_LOCATOR_AVAILABLE,
  exact_fingerprint_match = PP_EVIDENCE_EXACT_FINGERPRINT_MATCH,
  full_hash_match = PP_EVIDENCE_FULL_HASH_MATCH,
  partial_fingerprint_match = PP_EVIDENCE_PARTIAL_FINGERPRINT_MATCH,
  file_size_match = PP_EVIDENCE_FILE_SIZE_MATCH,
  file_name_match = PP_EVIDENCE_FILE_NAME_MATCH,
  relative_path_similarity = PP_EVIDENCE_RELATIVE_PATH_SIMILARITY,
  media_root_relation = PP_EVIDENCE_MEDIA_ROOT_RELATION,
  conflicting_candidate = PP_EVIDENCE_CONFLICTING_CANDIDATE,
  discovery_error = PP_EVIDENCE_DISCOVERY_ERROR,
};

struct Evidence final {
  EvidenceKind kind;
  std::optional<std::string> detail;
};

struct ResolutionCandidate final {
  std::string uri;
  std::uint16_t confidence_basis_points;
  std::vector<Evidence> evidence;
};

struct ResourceResolution final {
  Uuid resource_id;
  ResourceResolutionState state;
  std::vector<ResolutionCandidate> candidates;
  std::vector<Evidence> evidence;
};

struct AvailabilityIssue final {
  Uuid resource_id;
  bool required;
  AvailabilityIssueKind kind;
  std::vector<std::int64_t> frames;
};

struct RepresentationResolution final {
  Uuid representation_id;
  RepresentationAvailability availability;
  std::vector<ResourceResolution> resources;
  std::vector<AvailabilityIssue> issues;
};

namespace detail {

struct ErrorDeleter final {
  void operator()(pp_error_t *error) const noexcept { pp_error_release(error); }
};

using ErrorHandle = std::unique_ptr<pp_error_t, ErrorDeleter>;

struct ResolutionSetDeleter final {
  void operator()(pp_resolution_set_t *resolutions) const noexcept {
    pp_resolution_set_release(resolutions);
  }
};

using ResolutionSetHandle =
    std::unique_ptr<pp_resolution_set_t, ResolutionSetDeleter>;

struct RepresentationSetDeleter final {
  void operator()(pp_representation_set_t *representations) const noexcept {
    pp_representation_set_release(representations);
  }
};

using RepresentationSetHandle =
    std::unique_ptr<pp_representation_set_t, RepresentationSetDeleter>;

struct ExternalIdentifierSetDeleter final {
  void operator()(pp_external_identifier_set_t *identifiers) const noexcept {
    pp_external_identifier_set_release(identifiers);
  }
};

using ExternalIdentifierSetHandle = std::unique_ptr<
    pp_external_identifier_set_t, ExternalIdentifierSetDeleter>;

struct ObjectRefSetDeleter final {
  void operator()(pp_object_ref_set_t *objects) const noexcept {
    pp_object_ref_set_release(objects);
  }
};

using ObjectRefSetHandle =
    std::unique_ptr<pp_object_ref_set_t, ObjectRefSetDeleter>;

struct ActivitySetDeleter final {
  void operator()(pp_activity_set_t *activities) const noexcept {
    pp_activity_set_release(activities);
  }
};

using ActivitySetHandle =
    std::unique_ptr<pp_activity_set_t, ActivitySetDeleter>;

struct RevisionSetDeleter final {
  void operator()(pp_revision_set_t *revisions) const noexcept {
    pp_revision_set_release(revisions);
  }
};

using RevisionSetHandle =
    std::unique_ptr<pp_revision_set_t, RevisionSetDeleter>;

struct RevisionEventSetDeleter final {
  void operator()(pp_revision_event_set_t *events) const noexcept {
    pp_revision_event_set_release(events);
  }
};

using RevisionEventSetHandle =
    std::unique_ptr<pp_revision_event_set_t, RevisionEventSetDeleter>;

struct HostBindingDeleter final {
  void operator()(char *binding) const noexcept {
    pp_host_binding_release(binding);
  }
};

using HostBindingHandle = std::unique_ptr<char, HostBindingDeleter>;

inline void throw_if_error(pp_error_code_t status, pp_error_t *raw_error) {
  ErrorHandle error(raw_error);
  if (status == PP_OK) {
    return;
  }

  const char *raw_message = error ? pp_error_message(error.get()) : nullptr;
  std::string message =
      raw_message != nullptr ? raw_message : "PostProject operation failed";
  throw Error(static_cast<ErrorCode>(status), std::move(message));
}

inline std::string checked_string(std::string_view value,
                                  std::string_view label) {
  if (value.find('\0') != std::string_view::npos) {
    throw std::invalid_argument(std::string(label) +
                                " must not contain an embedded NUL");
  }
  return std::string(value);
}

inline Uuid uuid(const pp_uuid_t &value) {
  Uuid::Bytes bytes{};
  for (std::size_t index = 0; index < bytes.size(); ++index) {
    bytes[index] = value.bytes[index];
  }
  return Uuid(bytes);
}

inline pp_uuid_t native_uuid(const Uuid &value) {
  pp_uuid_t native{};
  for (std::size_t index = 0; index < value.bytes().size(); ++index) {
    native.bytes[index] = value.bytes()[index];
  }
  return native;
}

inline ObjectRef object_ref(const pp_object_ref_t &value) {
  return {static_cast<ObjectKind>(value.kind), uuid(value.id)};
}

inline pp_object_ref_t native_object_ref(const ObjectRef &value) {
  return {static_cast<pp_object_kind_t>(value.kind), native_uuid(value.id)};
}

inline std::optional<std::string> optional_string(const char *value) {
  return value != nullptr
             ? std::optional<std::string>(std::string(value))
             : std::nullopt;
}

inline std::optional<std::string>
checked_optional_string(const std::optional<std::string> &value,
                        std::string_view label) {
  return value.has_value()
             ? std::optional<std::string>(checked_string(*value, label))
             : std::nullopt;
}

inline Fingerprint representation_fingerprint(
    const pp_representation_set_t *representations,
    std::uint64_t representation_index, std::uint64_t fingerprint_index) {
  const char *algorithm = nullptr;
  std::uint16_t version = 0;
  const std::uint8_t *value = nullptr;
  std::uint64_t value_length = 0;
  pp_error_t *error = nullptr;
  const pp_error_code_t status = pp_representation_set_get_fingerprint(
      representations, representation_index, fingerprint_index, &algorithm,
      &version, &value, &value_length, &error);
  throw_if_error(status, error);
  std::vector<std::uint8_t> copied_value;
  if (value != nullptr) {
    copied_value.assign(value, value + value_length);
  }
  return {algorithm != nullptr ? std::string(algorithm) : std::string(), version,
          std::move(copied_value)};
}

inline Fingerprint resource_fingerprint(
    const pp_representation_set_t *representations,
    std::uint64_t representation_index, std::uint64_t resource_index,
    std::uint64_t fingerprint_index) {
  const char *algorithm = nullptr;
  std::uint16_t version = 0;
  const std::uint8_t *value = nullptr;
  std::uint64_t value_length = 0;
  pp_error_t *error = nullptr;
  const pp_error_code_t status =
      pp_representation_set_get_resource_fingerprint(
          representations, representation_index, resource_index,
          fingerprint_index, &algorithm, &version, &value, &value_length,
          &error);
  throw_if_error(status, error);
  std::vector<std::uint8_t> copied_value;
  if (value != nullptr) {
    copied_value.assign(value, value + value_length);
  }
  return {algorithm != nullptr ? std::string(algorithm) : std::string(), version,
          std::move(copied_value)};
}

inline Representation representation(
    const pp_representation_set_t *representations, std::uint64_t index) {
  pp_uuid_t id{};
  pp_uuid_t asset_id{};
  pp_representation_kind_t kind = 0;
  pp_content_structure_kind_t structure_kind = 0;
  std::uint64_t member_count = 0;
  std::uint64_t resource_count = 0;
  std::uint64_t fingerprint_count = 0;
  pp_error_t *error = nullptr;
  pp_error_code_t status = pp_representation_set_get(
      representations, index, &id, &asset_id, &kind, &structure_kind,
      &member_count, &resource_count, &fingerprint_count, &error);
  throw_if_error(status, error);

  std::vector<RepresentationMember> members;
  members.reserve(static_cast<std::size_t>(member_count));
  for (std::uint64_t member_index = 0; member_index < member_count;
       ++member_index) {
    pp_uuid_t resource_id{};
    const char *role = nullptr;
    std::uint8_t required = 0;
    error = nullptr;
    status = pp_representation_set_get_member(
        representations, index, member_index, &resource_id, &role, &required,
        &error);
    throw_if_error(status, error);
    members.push_back({uuid(resource_id), optional_string(role), required != 0});
  }

  std::optional<ImageSequenceDescriptor> image_sequence;
  if (structure_kind == PP_CONTENT_IMAGE_SEQUENCE) {
    const char *prefix = nullptr;
    const char *suffix = nullptr;
    std::uint8_t padding = 0;
    std::int64_t start = 0;
    std::int64_t end = 0;
    std::uint32_t step = 0;
    std::uint32_t rate_numerator = 0;
    std::uint32_t rate_denominator = 0;
    std::uint64_t missing_count = 0;
    error = nullptr;
    status = pp_representation_set_get_sequence(
        representations, index, &prefix, &suffix, &padding, &start, &end, &step,
        &rate_numerator, &rate_denominator, &missing_count, &error);
    throw_if_error(status, error);
    std::vector<std::int64_t> missing_frames;
    missing_frames.reserve(static_cast<std::size_t>(missing_count));
    for (std::uint64_t frame_index = 0; frame_index < missing_count;
         ++frame_index) {
      std::int64_t frame = 0;
      error = nullptr;
      status = pp_representation_set_get_sequence_missing_frame(
          representations, index, frame_index, &frame, &error);
      throw_if_error(status, error);
      missing_frames.push_back(frame);
    }
    image_sequence = ImageSequenceDescriptor{
        prefix != nullptr ? std::string(prefix) : std::string(),
        suffix != nullptr ? std::string(suffix) : std::string(),
        padding,
        start,
        end,
        step,
        rate_numerator,
        rate_denominator,
        std::move(missing_frames)};
  }

  std::vector<Fingerprint> fingerprints;
  fingerprints.reserve(static_cast<std::size_t>(fingerprint_count));
  for (std::uint64_t fingerprint_index = 0;
       fingerprint_index < fingerprint_count; ++fingerprint_index) {
    fingerprints.push_back(representation_fingerprint(
        representations, index, fingerprint_index));
  }

  std::vector<Resource> resources;
  resources.reserve(static_cast<std::size_t>(resource_count));
  for (std::uint64_t resource_index = 0; resource_index < resource_count;
       ++resource_index) {
    pp_uuid_t resource_id{};
    std::uint8_t has_file_facts = 0;
    std::uint64_t file_size = 0;
    std::uint8_t has_modified_at = 0;
    std::int64_t modified_at = 0;
    std::uint64_t locator_count = 0;
    std::uint64_t resource_fingerprint_count = 0;
    error = nullptr;
    status = pp_representation_set_get_resource(
        representations, index, resource_index, &resource_id, &has_file_facts,
        &file_size, &has_modified_at, &modified_at, &locator_count,
        &resource_fingerprint_count, &error);
    throw_if_error(status, error);

    std::vector<Fingerprint> resource_fingerprints;
    resource_fingerprints.reserve(
        static_cast<std::size_t>(resource_fingerprint_count));
    for (std::uint64_t fingerprint_index = 0;
         fingerprint_index < resource_fingerprint_count; ++fingerprint_index) {
      resource_fingerprints.push_back(resource_fingerprint(
          representations, index, resource_index, fingerprint_index));
    }

    std::vector<Locator> locators;
    locators.reserve(static_cast<std::size_t>(locator_count));
    for (std::uint64_t locator_index = 0; locator_index < locator_count;
         ++locator_index) {
      pp_uuid_t locator_id{};
      const char *uri = nullptr;
      pp_locator_availability_t availability = 0;
      std::uint8_t has_last_seen = 0;
      std::int64_t last_seen = 0;
      error = nullptr;
      status = pp_representation_set_get_locator(
          representations, index, resource_index, locator_index, &locator_id,
          &uri, &availability, &has_last_seen, &last_seen, &error);
      throw_if_error(status, error);
      locators.push_back(
          {uuid(locator_id), uri != nullptr ? std::string(uri) : std::string(),
           static_cast<LocatorAvailability>(availability),
           has_last_seen != 0
               ? std::optional<std::int64_t>(last_seen)
               : std::nullopt});
    }
    resources.push_back(
        {uuid(resource_id),
         has_file_facts != 0 ? std::optional<std::uint64_t>(file_size)
                             : std::nullopt,
         has_modified_at != 0 ? std::optional<std::int64_t>(modified_at)
                              : std::nullopt,
         std::move(resource_fingerprints), std::move(locators)});
  }

  return {uuid(id),
          uuid(asset_id),
          static_cast<RepresentationKind>(kind),
          static_cast<ContentStructureKind>(structure_kind),
          std::move(members),
          std::move(image_sequence),
          std::move(fingerprints),
          std::move(resources)};
}

inline Activity activity(const pp_activity_set_t *activities,
                         std::uint64_t index) {
  pp_uuid_t id{};
  const char *kind = nullptr;
  std::uint8_t has_started_at = 0;
  std::int64_t started_at = 0;
  std::uint8_t has_finished_at = 0;
  std::int64_t finished_at = 0;
  std::uint64_t input_count = 0;
  std::uint64_t output_count = 0;
  pp_error_t *error = nullptr;
  pp_error_code_t status = pp_activity_set_get(
      activities, index, &id, &kind, &has_started_at, &started_at,
      &has_finished_at, &finished_at, &input_count, &output_count, &error);
  throw_if_error(status, error);

  const char *tool_name = nullptr;
  const char *tool_version = nullptr;
  const char *tool_uri = nullptr;
  error = nullptr;
  status = pp_activity_set_get_tool(activities, index, &tool_name,
                                    &tool_version, &tool_uri, &error);
  throw_if_error(status, error);
  std::optional<ToolIdentity> tool;
  if (tool_name != nullptr) {
    tool = ToolIdentity{std::string(tool_name), optional_string(tool_version),
                        optional_string(tool_uri)};
  }

  const char *agent_name = nullptr;
  const char *agent_scheme = nullptr;
  const char *agent_value = nullptr;
  const char *agent_qualifier = nullptr;
  error = nullptr;
  status = pp_activity_set_get_agent(
      activities, index, &agent_name, &agent_scheme, &agent_value,
      &agent_qualifier, &error);
  throw_if_error(status, error);
  std::optional<AgentIdentity> agent;
  if (agent_name != nullptr || agent_scheme != nullptr) {
    std::optional<ExternalIdentifier> identifier;
    if (agent_scheme != nullptr && agent_value != nullptr) {
      identifier = ExternalIdentifier{std::string(agent_scheme),
                                      std::string(agent_value),
                                      optional_string(agent_qualifier)};
    }
    agent = AgentIdentity{optional_string(agent_name), std::move(identifier)};
  }

  std::vector<ActivityEdge> inputs;
  inputs.reserve(static_cast<std::size_t>(input_count));
  for (std::uint64_t edge_index = 0; edge_index < input_count; ++edge_index) {
    pp_uuid_t representation_id{};
    const char *role = nullptr;
    error = nullptr;
    status = pp_activity_set_get_input(activities, index, edge_index,
                                       &representation_id, &role, &error);
    throw_if_error(status, error);
    inputs.push_back({uuid(representation_id), optional_string(role)});
  }
  std::vector<ActivityEdge> outputs;
  outputs.reserve(static_cast<std::size_t>(output_count));
  for (std::uint64_t edge_index = 0; edge_index < output_count; ++edge_index) {
    pp_uuid_t representation_id{};
    const char *role = nullptr;
    error = nullptr;
    status = pp_activity_set_get_output(activities, index, edge_index,
                                        &representation_id, &role, &error);
    throw_if_error(status, error);
    outputs.push_back({uuid(representation_id), optional_string(role)});
  }

  return {uuid(id),
          kind != nullptr ? std::string(kind) : std::string(),
          has_started_at != 0 ? std::optional<std::int64_t>(started_at)
                              : std::nullopt,
          has_finished_at != 0 ? std::optional<std::int64_t>(finished_at)
                               : std::nullopt,
          std::move(tool),
          std::move(agent),
          std::move(inputs),
          std::move(outputs)};
}

inline Revision revision(const pp_revision_set_t *revisions,
                         std::uint64_t index) {
  pp_uuid_t id{};
  std::uint64_t sequence = 0;
  pp_uuid_t transaction_id{};
  std::int64_t committed_at = 0;
  const char *origin_name = nullptr;
  const char *origin_version = nullptr;
  const char *origin_uri = nullptr;
  const char *message = nullptr;
  pp_error_t *error = nullptr;
  const pp_error_code_t status = pp_revision_set_get(
      revisions, index, &id, &sequence, &transaction_id, &committed_at,
      &origin_name, &origin_version, &origin_uri, &message, &error);
  throw_if_error(status, error);
  std::optional<OriginIdentity> origin;
  if (origin_name != nullptr) {
    origin = OriginIdentity{std::string(origin_name),
                            optional_string(origin_version),
                            optional_string(origin_uri)};
  }
  return {uuid(id), sequence, uuid(transaction_id), committed_at,
          std::move(origin), optional_string(message)};
}

inline std::string required_event_string(const char *value,
                                         std::string_view label) {
  if (value == nullptr) {
    throw Error(ErrorCode::internal,
                "revision event is missing " + std::string(label));
  }
  return std::string(value);
}

inline RevisionEvent revision_event(const pp_revision_event_set_t *events,
                                    std::uint64_t index) {
  pp_revision_event_t event{};
  pp_error_t *error = nullptr;
  const pp_error_code_t status =
      pp_revision_event_set_get(events, index, &event, &error);
  throw_if_error(status, error);
  switch (event.kind) {
  case PP_REVISION_ASSET_IMPORTED:
    return {event.position, AssetImportedEvent{uuid(event.asset_id)}};
  case PP_REVISION_REPRESENTATION_ADDED:
    return {event.position,
            RepresentationAddedEvent{uuid(event.asset_id),
                                     uuid(event.representation_id)}};
  case PP_REVISION_RESOURCE_ADDED:
    return {event.position, ResourceAddedEvent{uuid(event.resource_id)}};
  case PP_REVISION_REPRESENTATION_RESOURCE_ADDED:
    return {event.position,
            RepresentationResourceAddedEvent{
                uuid(event.representation_id), uuid(event.resource_id),
                event.structural_position}};
  case PP_REVISION_LOCATOR_ADDED:
    return {event.position,
            LocatorAddedEvent{uuid(event.resource_id),
                              uuid(event.locator_id)}};
  case PP_REVISION_MEDIA_ROOT_ADDED:
    return {event.position,
            MediaRootAddedEvent{uuid(event.media_root_id)}};
  case PP_REVISION_EXTERNAL_IDENTIFIER_ADDED:
    return {event.position,
            ExternalIdentifierAddedEvent{
                object_ref(event.target),
                {required_event_string(event.identifier_scheme,
                                       "identifier scheme"),
                 required_event_string(event.identifier_value,
                                       "identifier value"),
                 optional_string(event.identifier_qualifier)}}};
  case PP_REVISION_EXTERNAL_IDENTIFIER_REMOVED:
    return {event.position,
            ExternalIdentifierRemovedEvent{
                object_ref(event.target),
                {required_event_string(event.identifier_scheme,
                                       "identifier scheme"),
                 required_event_string(event.identifier_value,
                                       "identifier value"),
                 optional_string(event.identifier_qualifier)}}};
  case PP_REVISION_METADATA_ADDED_OR_REPLACED:
    return {event.position,
            MetadataAddedOrReplacedEvent{
                object_ref(event.target),
                required_event_string(event.vocabulary, "metadata vocabulary"),
                required_event_string(event.property, "metadata property")}};
  case PP_REVISION_METADATA_REMOVED:
    return {event.position,
            MetadataRemovedEvent{
                object_ref(event.target),
                required_event_string(event.vocabulary, "metadata vocabulary"),
                required_event_string(event.property, "metadata property")}};
  case PP_REVISION_ACTIVITY_CREATED:
    return {event.position,
            ActivityCreatedEvent{
                uuid(event.activity_id),
                required_event_string(event.activity_kind, "activity kind")}};
  case PP_REVISION_ACTIVITY_INPUT_ADDED:
    return {event.position,
            ActivityInputAddedEvent{uuid(event.activity_id),
                                    uuid(event.representation_id),
                                    optional_string(event.role)}};
  case PP_REVISION_ACTIVITY_OUTPUT_ADDED:
    return {event.position,
            ActivityOutputAddedEvent{uuid(event.activity_id),
                                     uuid(event.representation_id),
                                     optional_string(event.role)}};
  default:
    throw Error(ErrorCode::internal,
                "revision event has an unknown semantic kind");
  }
}

inline Evidence resource_evidence(const pp_resolution_set_t *resolutions,
                                  std::uint64_t representation_index,
                                  std::uint64_t resource_index,
                                  std::uint64_t evidence_index) {
  pp_evidence_kind_t kind = 0;
  const char *detail = nullptr;
  pp_error_t *error = nullptr;
  const pp_error_code_t status = pp_resolution_set_get_resource_evidence(
      resolutions, representation_index, resource_index, evidence_index, &kind,
      &detail, &error);
  throw_if_error(status, error);
  return {static_cast<EvidenceKind>(kind),
          detail != nullptr
              ? std::optional<std::string>(std::string(detail))
              : std::nullopt};
}

inline Evidence candidate_evidence(const pp_resolution_set_t *resolutions,
                                   std::uint64_t representation_index,
                                   std::uint64_t resource_index,
                                   std::uint64_t candidate_index,
                                   std::uint64_t evidence_index) {
  pp_evidence_kind_t kind = 0;
  const char *detail = nullptr;
  pp_error_t *error = nullptr;
  const pp_error_code_t status = pp_resolution_set_get_candidate_evidence(
      resolutions, representation_index, resource_index, candidate_index,
      evidence_index, &kind, &detail, &error);
  throw_if_error(status, error);
  return {static_cast<EvidenceKind>(kind),
          detail != nullptr
              ? std::optional<std::string>(std::string(detail))
              : std::nullopt};
}

} // namespace detail

inline std::string HostObjectBinding::toString() const {
  const pp_uuid_t native_production_id = detail::native_uuid(production_id);
  const pp_object_ref_t native_object = detail::native_object_ref(object);
  char *binding = nullptr;
  pp_error_t *error = nullptr;
  const pp_error_code_t status = pp_host_binding_format(
      &native_production_id, &native_object, &binding, &error);
  detail::throw_if_error(status, error);
  detail::HostBindingHandle owned(binding);
  return owned != nullptr ? std::string(owned.get()) : std::string();
}

inline HostObjectBinding
HostObjectBinding::fromString(std::string_view value) {
  const std::string checked = detail::checked_string(value, "host binding");
  pp_uuid_t production_id{};
  pp_object_ref_t object{};
  pp_error_t *error = nullptr;
  const pp_error_code_t status = pp_host_binding_parse(
      checked.c_str(), &production_id, &object, &error);
  detail::throw_if_error(status, error);
  return {detail::uuid(production_id), detail::object_ref(object)};
}

// Move-only immutable input that can be reused across transaction calls.
class MetadataInput final {
public:
  [[nodiscard]] static MetadataInput plainString(std::string_view value) {
    const std::string native = detail::checked_string(value, "metadata string");
    pp_metadata_input_t *input = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_metadata_input_create_string(
        native.c_str(), nullptr, &input, &error);
    return checked(status, input, error);
  }

  [[nodiscard]] static MetadataInput languageString(
      std::string_view value, std::string_view language) {
    const std::string native = detail::checked_string(value, "metadata string");
    const std::string native_language =
        detail::checked_string(language, "metadata language");
    pp_metadata_input_t *input = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_metadata_input_create_string(
        native.c_str(), native_language.c_str(), &input, &error);
    return checked(status, input, error);
  }

  [[nodiscard]] static MetadataInput signedInteger(std::int64_t value) {
    pp_metadata_input_t *input = nullptr;
    pp_error_t *error = nullptr;
    return checked(pp_metadata_input_create_i64(value, &input, &error), input,
                   error);
  }

  [[nodiscard]] static MetadataInput unsignedInteger(std::uint64_t value) {
    pp_metadata_input_t *input = nullptr;
    pp_error_t *error = nullptr;
    return checked(pp_metadata_input_create_u64(value, &input, &error), input,
                   error);
  }

  [[nodiscard]] static MetadataInput decimal(std::string_view coefficient,
                                             std::uint32_t scale) {
    const std::string native =
        detail::checked_string(coefficient, "decimal coefficient");
    pp_metadata_input_t *input = nullptr;
    pp_error_t *error = nullptr;
    return checked(pp_metadata_input_create_decimal(
                       native.c_str(), scale, &input, &error),
                   input, error);
  }

  [[nodiscard]] static MetadataInput boolean(bool value) {
    pp_metadata_input_t *input = nullptr;
    pp_error_t *error = nullptr;
    return checked(pp_metadata_input_create_bool(value ? 1 : 0, &input, &error),
                   input, error);
  }

  [[nodiscard]] static MetadataInput timestamp(std::int64_t unix_micros) {
    pp_metadata_input_t *input = nullptr;
    pp_error_t *error = nullptr;
    return checked(pp_metadata_input_create_timestamp(unix_micros, &input,
                                                      &error),
                   input, error);
  }

  [[nodiscard]] static MetadataInput uri(std::string_view value) {
    const std::string native = detail::checked_string(value, "metadata URI");
    pp_metadata_input_t *input = nullptr;
    pp_error_t *error = nullptr;
    return checked(
        pp_metadata_input_create_uri(native.c_str(), &input, &error), input,
        error);
  }

  [[nodiscard]] static MetadataInput
  bytes(const std::vector<std::uint8_t> &value) {
    pp_metadata_input_t *input = nullptr;
    pp_error_t *error = nullptr;
    return checked(pp_metadata_input_create_bytes(
                       value.data(), static_cast<std::uint64_t>(value.size()),
                       &input, &error),
                   input, error);
  }

  [[nodiscard]] static MetadataInput rational(std::int64_t numerator,
                                              std::uint64_t denominator) {
    pp_metadata_input_t *input = nullptr;
    pp_error_t *error = nullptr;
    return checked(pp_metadata_input_create_rational(
                       numerator, denominator, &input, &error),
                   input, error);
  }

  [[nodiscard]] static MetadataInput reference(const ObjectRef &target) {
    const pp_object_ref_t native = detail::native_object_ref(target);
    pp_metadata_input_t *input = nullptr;
    pp_error_t *error = nullptr;
    return checked(
        pp_metadata_input_create_reference(&native, &input, &error), input,
        error);
  }

  MetadataInput(const MetadataInput &) = delete;
  MetadataInput &operator=(const MetadataInput &) = delete;

  MetadataInput(MetadataInput &&other) noexcept
      : input_(std::exchange(other.input_, nullptr)) {}

  MetadataInput &operator=(MetadataInput &&other) noexcept {
    if (this != &other) {
      pp_metadata_input_release(input_);
      input_ = std::exchange(other.input_, nullptr);
    }
    return *this;
  }

  ~MetadataInput() { pp_metadata_input_release(input_); }

private:
  friend class Transaction;

  explicit MetadataInput(pp_metadata_input_t *input) noexcept : input_(input) {}

  [[nodiscard]] static MetadataInput checked(pp_error_code_t status,
                                             pp_metadata_input_t *input,
                                             pp_error_t *error) {
    detail::throw_if_error(status, error);
    return MetadataInput(input);
  }

  pp_metadata_input_t *input_;
};

// Move-only and caller-serialized. Do not call one Transaction concurrently.
class Transaction final {
public:
  void setRevisionContext(const RevisionContext &context) {
    const std::optional<std::string> origin_name =
        context.origin.has_value()
            ? std::optional<std::string>(checked_origin_name(*context.origin))
            : std::nullopt;
    const std::optional<std::string> origin_version =
        context.origin.has_value()
            ? detail::checked_optional_string(context.origin->version,
                                              "origin version")
            : std::nullopt;
    const std::optional<std::string> origin_uri =
        context.origin.has_value()
            ? detail::checked_optional_string(context.origin->uri,
                                              "origin URI")
            : std::nullopt;
    const std::optional<std::string> message =
        detail::checked_optional_string(context.message, "revision message");
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_transaction_set_revision_context(
        transaction_, origin_name.has_value() ? origin_name->c_str() : nullptr,
        origin_version.has_value() ? origin_version->c_str() : nullptr,
        origin_uri.has_value() ? origin_uri->c_str() : nullptr,
        message.has_value() ? message->c_str() : nullptr, &error);
    detail::throw_if_error(status, error);
  }

  Uuid importMedia(std::string_view path) {
    return import_media_impl(path, nullptr);
  }

  Uuid importMedia(std::string_view path, std::string_view display_name) {
    const std::string name =
        detail::checked_string(display_name, "display_name");
    return import_media_impl(path, name.c_str());
  }

  Uuid addMediaRoot(std::string_view path, std::int32_t priority = 0) {
    return add_media_root_impl(path, nullptr, priority);
  }

  Uuid addMediaRoot(std::string_view path, std::string_view label,
                    std::int32_t priority = 0) {
    const std::string native_label = detail::checked_string(label, "label");
    return add_media_root_impl(path, native_label.c_str(), priority);
  }

  void confirmLocator(const Uuid &resource_id, std::string_view uri) {
    const pp_uuid_t id = detail::native_uuid(resource_id);
    const std::string native_uri = detail::checked_string(uri, "uri");
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_transaction_confirm_locator(
        transaction_, &id, native_uri.c_str(), &error);
    detail::throw_if_error(status, error);
  }

  void addExternalIdentifier(const ObjectRef &target,
                             const ExternalIdentifier &identifier) {
    mutate_external_identifier(false, target, identifier);
  }

  void removeExternalIdentifier(const ObjectRef &target,
                                const ExternalIdentifier &identifier) {
    mutate_external_identifier(true, target, identifier);
  }

  void addMetadataValue(const ObjectRef &target, std::string_view vocabulary,
                        std::string_view property,
                        const MetadataInput &input) {
    const pp_object_ref_t native_target = detail::native_object_ref(target);
    const std::string native_vocabulary =
        detail::checked_string(vocabulary, "metadata vocabulary");
    const std::string native_property =
        detail::checked_string(property, "metadata property");
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_transaction_add_metadata_value(
        transaction_, &native_target, native_vocabulary.c_str(),
        native_property.c_str(), input.input_, &error);
    detail::throw_if_error(status, error);
  }

  Uuid createActivity(const ActivitySpec &spec) {
    const std::string kind = detail::checked_string(spec.kind, "kind");
    std::vector<pp_activity_edge_t> inputs;
    inputs.reserve(spec.inputs.size());
    for (const ActivityEdge &edge : spec.inputs) {
      if (edge.role.has_value()) {
        static_cast<void>(detail::checked_string(*edge.role, "input role"));
      }
      inputs.push_back(
          {detail::native_uuid(edge.representation_id),
           edge.role.has_value() ? edge.role->c_str() : nullptr});
    }
    std::vector<pp_activity_edge_t> outputs;
    outputs.reserve(spec.outputs.size());
    for (const ActivityEdge &edge : spec.outputs) {
      if (edge.role.has_value()) {
        static_cast<void>(detail::checked_string(*edge.role, "output role"));
      }
      outputs.push_back(
          {detail::native_uuid(edge.representation_id),
           edge.role.has_value() ? edge.role->c_str() : nullptr});
    }

    const std::optional<std::string> tool_name =
        spec.tool.has_value()
            ? std::optional<std::string>(
                  detail::checked_string(spec.tool->name, "tool name"))
            : std::nullopt;
    const std::optional<std::string> tool_version =
        spec.tool.has_value()
            ? detail::checked_optional_string(spec.tool->version,
                                              "tool version")
            : std::nullopt;
    const std::optional<std::string> tool_uri =
        spec.tool.has_value()
            ? detail::checked_optional_string(spec.tool->uri, "tool URI")
            : std::nullopt;
    const std::optional<std::string> agent_name =
        spec.agent.has_value()
            ? detail::checked_optional_string(spec.agent->name, "agent name")
            : std::nullopt;
    const std::optional<ExternalIdentifier> identifier =
        spec.agent.has_value() ? spec.agent->identifier : std::nullopt;
    const std::optional<std::string> agent_scheme =
        identifier.has_value()
            ? std::optional<std::string>(detail::checked_string(
                  identifier->scheme, "agent identifier scheme"))
            : std::nullopt;
    const std::optional<std::string> agent_value =
        identifier.has_value()
            ? std::optional<std::string>(detail::checked_string(
                  identifier->value, "agent identifier value"))
            : std::nullopt;
    const std::optional<std::string> agent_qualifier =
        identifier.has_value()
            ? detail::checked_optional_string(identifier->qualifier,
                                              "agent identifier qualifier")
            : std::nullopt;

    pp_uuid_t activity_id{};
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_transaction_create_activity(
        transaction_, kind.c_str(), inputs.data(),
        static_cast<std::uint64_t>(inputs.size()), outputs.data(),
        static_cast<std::uint64_t>(outputs.size()),
        spec.started_at_unix_micros.has_value()
            ? &*spec.started_at_unix_micros
            : nullptr,
        spec.finished_at_unix_micros.has_value()
            ? &*spec.finished_at_unix_micros
            : nullptr,
        tool_name.has_value() ? tool_name->c_str() : nullptr,
        tool_version.has_value() ? tool_version->c_str() : nullptr,
        tool_uri.has_value() ? tool_uri->c_str() : nullptr,
        agent_name.has_value() ? agent_name->c_str() : nullptr,
        agent_scheme.has_value() ? agent_scheme->c_str() : nullptr,
        agent_value.has_value() ? agent_value->c_str() : nullptr,
        agent_qualifier.has_value() ? agent_qualifier->c_str() : nullptr,
        &activity_id, &error);
    detail::throw_if_error(status, error);
    return detail::uuid(activity_id);
  }

  void commit() {
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_transaction_commit(transaction_, &error);
    detail::throw_if_error(status, error);
  }

  void rollback() {
    pp_error_t *error = nullptr;
    const pp_error_code_t status =
        pp_transaction_rollback(transaction_, &error);
    detail::throw_if_error(status, error);
  }

  Transaction(const Transaction &) = delete;
  Transaction &operator=(const Transaction &) = delete;

  Transaction(Transaction &&other) noexcept
      : transaction_(std::exchange(other.transaction_, nullptr)) {}

  Transaction &operator=(Transaction &&other) noexcept {
    if (this != &other) {
      pp_transaction_release(transaction_);
      transaction_ = std::exchange(other.transaction_, nullptr);
    }
    return *this;
  }

  ~Transaction() { pp_transaction_release(transaction_); }

  [[nodiscard]] explicit operator bool() const noexcept {
    return transaction_ != nullptr;
  }

private:
  friend class Production;

  explicit Transaction(pp_transaction_t *transaction) noexcept
      : transaction_(transaction) {}

  static std::string checked_origin_name(const OriginIdentity &origin) {
    return detail::checked_string(origin.name, "origin name");
  }

  Uuid import_media_impl(std::string_view path, const char *display_name) {
    const std::string native_path = detail::checked_string(path, "path");
    pp_uuid_t value{};
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_transaction_import_media(
        transaction_, native_path.c_str(), display_name, &value, &error);
    detail::throw_if_error(status, error);
    return detail::uuid(value);
  }

  Uuid add_media_root_impl(std::string_view path, const char *label,
                           std::int32_t priority) {
    const std::string native_path = detail::checked_string(path, "path");
    pp_uuid_t value{};
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_transaction_add_media_root(
        transaction_, native_path.c_str(), label, priority, &value, &error);
    detail::throw_if_error(status, error);
    return detail::uuid(value);
  }

  void mutate_external_identifier(bool remove, const ObjectRef &target,
                                  const ExternalIdentifier &identifier) {
    const pp_object_ref_t native_target = detail::native_object_ref(target);
    const std::string scheme =
        detail::checked_string(identifier.scheme, "scheme");
    const std::string value = detail::checked_string(identifier.value, "value");
    const std::optional<std::string> qualifier =
        identifier.qualifier.has_value()
            ? std::optional<std::string>(detail::checked_string(
                  *identifier.qualifier, "qualifier"))
            : std::nullopt;
    pp_error_t *error = nullptr;
    const pp_error_code_t status = remove
                                       ? pp_transaction_remove_external_identifier(
                                             transaction_, &native_target,
                                             scheme.c_str(), value.c_str(),
                                             qualifier.has_value()
                                                 ? qualifier->c_str()
                                                 : nullptr,
                                             &error)
                                       : pp_transaction_add_external_identifier(
                                             transaction_, &native_target,
                                             scheme.c_str(), value.c_str(),
                                             qualifier.has_value()
                                                 ? qualifier->c_str()
                                                 : nullptr,
                                             &error);
    detail::throw_if_error(status, error);
  }

  pp_transaction_t *transaction_ = nullptr;
};

// Move-only owner of a thread-safe native handle. Concurrent const calls are
// supported while ownership operations and destruction remain serialized.
class Production final {
public:
  static Production create(std::string_view path) {
    return create_impl(path, nullptr);
  }

  static Production create(std::string_view path, std::string_view display_name) {
    const std::string name =
        detail::checked_string(display_name, "display_name");
    return create_impl(path, name.c_str());
  }

  static Production open(std::string_view path) {
    const std::string native_path = detail::checked_string(path, "path");
    pp_production_t *production = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status =
        pp_production_open(native_path.c_str(), &production, &error);
    detail::throw_if_error(status, error);
    return Production(production);
  }

  Production(const Production &) = delete;
  Production &operator=(const Production &) = delete;

  Production(Production &&other) noexcept
      : production_(std::exchange(other.production_, nullptr)) {}

  Production &operator=(Production &&other) noexcept {
    if (this != &other) {
      pp_production_release(production_);
      production_ = std::exchange(other.production_, nullptr);
    }
    return *this;
  }

  ~Production() { pp_production_release(production_); }

  [[nodiscard]] Uuid id() const {
    pp_uuid_t value{};
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_production_id(production_, &value, &error);
    detail::throw_if_error(status, error);

    return detail::uuid(value);
  }

  [[nodiscard]] bool containsAsset(const Uuid &asset_id) const {
    const pp_uuid_t value = detail::native_uuid(asset_id);
    std::uint8_t exists = 0;
    pp_error_t *error = nullptr;
    const pp_error_code_t status =
        pp_production_asset_exists(production_, &value, &exists, &error);
    detail::throw_if_error(status, error);
    return exists != 0;
  }

  [[nodiscard]] std::vector<Representation>
  representations(const Uuid &asset_id) const {
    const pp_uuid_t native_asset_id = detail::native_uuid(asset_id);
    pp_representation_set_t *raw_representations = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_production_representations(
        production_, &native_asset_id, &raw_representations, &error);
    detail::throw_if_error(status, error);
    detail::RepresentationSetHandle representations(raw_representations);

    std::vector<Representation> result;
    const std::uint64_t count =
        pp_representation_set_count(representations.get());
    result.reserve(static_cast<std::size_t>(count));
    for (std::uint64_t index = 0; index < count; ++index) {
      result.push_back(detail::representation(representations.get(), index));
    }
    return result;
  }

  [[nodiscard]] std::vector<ExternalIdentifier>
  externalIdentifiers(const ObjectRef &target) const {
    const pp_object_ref_t native_target = detail::native_object_ref(target);
    pp_external_identifier_set_t *raw_identifiers = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_production_external_identifiers(
        production_, &native_target, &raw_identifiers, &error);
    detail::throw_if_error(status, error);
    detail::ExternalIdentifierSetHandle identifiers(raw_identifiers);

    std::vector<ExternalIdentifier> result;
    const std::uint64_t count =
        pp_external_identifier_set_count(identifiers.get());
    result.reserve(static_cast<std::size_t>(count));
    for (std::uint64_t index = 0; index < count; ++index) {
      const char *scheme = nullptr;
      const char *value = nullptr;
      const char *qualifier = nullptr;
      pp_error_t *item_error = nullptr;
      const pp_error_code_t item_status = pp_external_identifier_set_get(
          identifiers.get(), index, &scheme, &value, &qualifier, &item_error);
      detail::throw_if_error(item_status, item_error);
      result.push_back(
          {scheme != nullptr ? std::string(scheme) : std::string(),
           value != nullptr ? std::string(value) : std::string(),
           qualifier != nullptr
               ? std::optional<std::string>(std::string(qualifier))
               : std::nullopt});
    }
    return result;
  }

  [[nodiscard]] std::vector<ObjectRef>
  findByExternalIdentifier(std::string_view scheme,
                           std::string_view value) const {
    const std::string native_scheme =
        detail::checked_string(scheme, "scheme");
    const std::string native_value = detail::checked_string(value, "value");
    pp_object_ref_set_t *raw_objects = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_production_find_by_external_identifier(
        production_, native_scheme.c_str(), native_value.c_str(), &raw_objects,
        &error);
    detail::throw_if_error(status, error);
    detail::ObjectRefSetHandle objects(raw_objects);

    std::vector<ObjectRef> result;
    const std::uint64_t count = pp_object_ref_set_count(objects.get());
    result.reserve(static_cast<std::size_t>(count));
    for (std::uint64_t index = 0; index < count; ++index) {
      pp_object_ref_t object{};
      pp_error_t *item_error = nullptr;
      const pp_error_code_t item_status =
          pp_object_ref_set_get(objects.get(), index, &object, &item_error);
      detail::throw_if_error(item_status, item_error);
      result.push_back(detail::object_ref(object));
    }
    return result;
  }

  [[nodiscard]] std::vector<RepresentationResolution>
  resolveAsset(const Uuid &asset_id) const {
    const pp_uuid_t value = detail::native_uuid(asset_id);
    pp_resolution_set_t *raw_resolutions = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_production_resolve_asset(
        production_, &value, &raw_resolutions, &error);
    detail::throw_if_error(status, error);
    detail::ResolutionSetHandle resolutions(raw_resolutions);

    std::vector<RepresentationResolution> result;
    const std::uint64_t count =
        pp_resolution_set_representation_count(resolutions.get());
    for (std::uint64_t representation_index = 0;
         representation_index < count; ++representation_index) {
      pp_uuid_t representation_id{};
      pp_representation_availability_t availability = 0;
      std::uint64_t resource_count = 0;
      std::uint64_t issue_count = 0;
      pp_error_t *item_error = nullptr;
      const pp_error_code_t item_status =
          pp_resolution_set_get_representation(
              resolutions.get(), representation_index, &representation_id,
              &availability, &resource_count, &issue_count, &item_error);
      detail::throw_if_error(item_status, item_error);

      std::vector<ResourceResolution> resources;
      for (std::uint64_t resource_index = 0; resource_index < resource_count;
           ++resource_index) {
        pp_uuid_t resource_id{};
        pp_resource_resolution_state_t state = 0;
        std::uint64_t candidate_count = 0;
        std::uint64_t evidence_count = 0;
        pp_error_t *resource_error = nullptr;
        const pp_error_code_t resource_status = pp_resolution_set_get_resource(
            resolutions.get(), representation_index, resource_index,
            &resource_id, &state, &candidate_count, &evidence_count,
            &resource_error);
        detail::throw_if_error(resource_status, resource_error);

        std::vector<ResolutionCandidate> candidates;
        for (std::uint64_t candidate_index = 0;
             candidate_index < candidate_count; ++candidate_index) {
          const char *uri = nullptr;
          std::uint16_t confidence = 0;
          std::uint64_t candidate_evidence_count = 0;
          pp_error_t *candidate_error = nullptr;
          const pp_error_code_t candidate_status =
              pp_resolution_set_get_candidate(
                  resolutions.get(), representation_index, resource_index,
                  candidate_index, &uri, &confidence, &candidate_evidence_count,
                  &candidate_error);
          detail::throw_if_error(candidate_status, candidate_error);

          std::vector<Evidence> evidence;
          for (std::uint64_t evidence_index = 0;
               evidence_index < candidate_evidence_count; ++evidence_index) {
            evidence.push_back(detail::candidate_evidence(
                resolutions.get(), representation_index, resource_index,
                candidate_index, evidence_index));
          }
          candidates.push_back(
              {uri != nullptr ? std::string(uri) : std::string(), confidence,
               std::move(evidence)});
        }

        std::vector<Evidence> evidence;
        for (std::uint64_t evidence_index = 0;
             evidence_index < evidence_count; ++evidence_index) {
          evidence.push_back(detail::resource_evidence(
              resolutions.get(), representation_index, resource_index,
              evidence_index));
        }
        resources.push_back({detail::uuid(resource_id),
                             static_cast<ResourceResolutionState>(state),
                             std::move(candidates), std::move(evidence)});
      }

      std::vector<AvailabilityIssue> issues;
      for (std::uint64_t issue_index = 0; issue_index < issue_count;
           ++issue_index) {
        pp_uuid_t resource_id{};
        std::uint8_t required = 0;
        pp_availability_issue_kind_t kind = 0;
        std::uint64_t frame_count = 0;
        pp_error_t *issue_error = nullptr;
        const pp_error_code_t issue_status = pp_resolution_set_get_issue(
            resolutions.get(), representation_index, issue_index, &resource_id,
            &required, &kind, &frame_count, &issue_error);
        detail::throw_if_error(issue_status, issue_error);
        std::vector<std::int64_t> frames;
        for (std::uint64_t frame_index = 0; frame_index < frame_count;
             ++frame_index) {
          std::int64_t frame = 0;
          pp_error_t *frame_error = nullptr;
          const pp_error_code_t frame_status =
              pp_resolution_set_get_issue_frame(
                  resolutions.get(), representation_index, issue_index,
                  frame_index, &frame, &frame_error);
          detail::throw_if_error(frame_status, frame_error);
          frames.push_back(frame);
        }
        issues.push_back({detail::uuid(resource_id), required != 0,
                          static_cast<AvailabilityIssueKind>(kind),
                          std::move(frames)});
      }
      result.push_back({detail::uuid(representation_id),
                        static_cast<RepresentationAvailability>(availability),
                        std::move(resources), std::move(issues)});
    }
    return result;
  }

  [[nodiscard]] std::vector<Activity> activities() const {
    pp_activity_set_t *raw_activities = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status =
        pp_production_activities(production_, &raw_activities, &error);
    detail::throw_if_error(status, error);
    detail::ActivitySetHandle activities(raw_activities);

    std::vector<Activity> result;
    const std::uint64_t count = pp_activity_set_count(activities.get());
    result.reserve(static_cast<std::size_t>(count));
    for (std::uint64_t index = 0; index < count; ++index) {
      result.push_back(detail::activity(activities.get(), index));
    }
    return result;
  }

  [[nodiscard]] std::vector<Activity>
  activitiesProducing(const Uuid &representation_id) const {
    return activities_for_representation(
        representation_id, pp_production_activities_producing);
  }

  [[nodiscard]] std::vector<Activity>
  activitiesConsuming(const Uuid &representation_id) const {
    return activities_for_representation(
        representation_id, pp_production_activities_consuming);
  }

  [[nodiscard]] std::vector<Uuid>
  ancestors(const Uuid &representation_id) const {
    return provenance_relatives(representation_id,
                                pp_production_provenance_ancestors);
  }

  [[nodiscard]] std::vector<Uuid>
  descendants(const Uuid &representation_id) const {
    return provenance_relatives(representation_id,
                                pp_production_provenance_descendants);
  }

  [[nodiscard]] std::optional<Revision> latestRevision() const {
    pp_revision_set_t *raw_revisions = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status =
        pp_production_latest_revision(production_, &raw_revisions, &error);
    detail::throw_if_error(status, error);
    detail::RevisionSetHandle revisions(raw_revisions);
    if (pp_revision_set_count(revisions.get()) == 0) {
      return std::nullopt;
    }
    return detail::revision(revisions.get(), 0);
  }

  [[nodiscard]] std::vector<Revision>
  changesSince(std::uint64_t sequence, std::uint32_t limit = 100) const {
    pp_revision_set_t *raw_revisions = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_production_changes_since(
        production_, sequence, limit, &raw_revisions, &error);
    detail::throw_if_error(status, error);
    detail::RevisionSetHandle revisions(raw_revisions);
    std::vector<Revision> result;
    const std::uint64_t count = pp_revision_set_count(revisions.get());
    result.reserve(static_cast<std::size_t>(count));
    for (std::uint64_t index = 0; index < count; ++index) {
      result.push_back(detail::revision(revisions.get(), index));
    }
    return result;
  }

  [[nodiscard]] std::vector<RevisionEvent>
  revisionEvents(const Uuid &revision_id) const {
    const pp_uuid_t native_id = detail::native_uuid(revision_id);
    pp_revision_event_set_t *raw_events = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_production_revision_events(
        production_, &native_id, &raw_events, &error);
    detail::throw_if_error(status, error);
    detail::RevisionEventSetHandle events(raw_events);
    std::vector<RevisionEvent> result;
    const std::uint64_t count = pp_revision_event_set_count(events.get());
    result.reserve(static_cast<std::size_t>(count));
    for (std::uint64_t index = 0; index < count; ++index) {
      result.push_back(detail::revision_event(events.get(), index));
    }
    return result;
  }

  [[nodiscard]] Transaction beginTransaction() {
    pp_transaction_t *transaction = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status =
        pp_production_begin_transaction(production_, &transaction, &error);
    detail::throw_if_error(status, error);
    return Transaction(transaction);
  }

  [[nodiscard]] explicit operator bool() const noexcept {
    return production_ != nullptr;
  }

private:
  using ActivityQuery = pp_error_code_t (*)(
      const pp_production_t *, const pp_uuid_t *, pp_activity_set_t **,
      pp_error_t **);
  using ProvenanceQuery = pp_error_code_t (*)(
      const pp_production_t *, const pp_uuid_t *, pp_object_ref_set_t **,
      pp_error_t **);

  explicit Production(pp_production_t *production) noexcept : production_(production) {}

  [[nodiscard]] std::vector<Activity>
  activities_for_representation(const Uuid &representation_id,
                                ActivityQuery query) const {
    const pp_uuid_t native_id = detail::native_uuid(representation_id);
    pp_activity_set_t *raw_activities = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status =
        query(production_, &native_id, &raw_activities, &error);
    detail::throw_if_error(status, error);
    detail::ActivitySetHandle activities(raw_activities);

    std::vector<Activity> result;
    const std::uint64_t count = pp_activity_set_count(activities.get());
    result.reserve(static_cast<std::size_t>(count));
    for (std::uint64_t index = 0; index < count; ++index) {
      result.push_back(detail::activity(activities.get(), index));
    }
    return result;
  }

  [[nodiscard]] std::vector<Uuid>
  provenance_relatives(const Uuid &representation_id,
                       ProvenanceQuery query) const {
    const pp_uuid_t native_id = detail::native_uuid(representation_id);
    pp_object_ref_set_t *raw_objects = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status =
        query(production_, &native_id, &raw_objects, &error);
    detail::throw_if_error(status, error);
    detail::ObjectRefSetHandle objects(raw_objects);

    std::vector<Uuid> result;
    const std::uint64_t count = pp_object_ref_set_count(objects.get());
    result.reserve(static_cast<std::size_t>(count));
    for (std::uint64_t index = 0; index < count; ++index) {
      pp_object_ref_t object{};
      pp_error_t *item_error = nullptr;
      const pp_error_code_t item_status =
          pp_object_ref_set_get(objects.get(), index, &object, &item_error);
      detail::throw_if_error(item_status, item_error);
      if (object.kind != PP_OBJECT_REPRESENTATION) {
        throw Error(ErrorCode::internal,
                    "provenance query returned a non-representation object");
      }
      result.push_back(detail::uuid(object.id));
    }
    return result;
  }

  static Production create_impl(std::string_view path, const char *display_name) {
    const std::string native_path = detail::checked_string(path, "path");
    pp_production_t *production = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status =
        pp_production_create(native_path.c_str(), display_name, &production, &error);
    detail::throw_if_error(status, error);
    return Production(production);
  }

  pp_production_t *production_ = nullptr;
};

[[nodiscard]] inline std::uint32_t abi_version() noexcept {
  return pp_abi_version();
}

} // namespace postproject

#endif
