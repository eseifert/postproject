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
  project = PP_OBJECT_PROJECT,
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

struct ExternalIdentifier final {
  std::string scheme;
  std::string value;
  std::optional<std::string> qualifier;
};

enum class ResolutionState : std::uint32_t {
  online_at_known_locator = PP_RESOLUTION_ONLINE_AT_KNOWN_LOCATOR,
  resolved_exact = PP_RESOLUTION_RESOLVED_EXACT,
  resolved_probable = PP_RESOLUTION_RESOLVED_PROBABLE,
  missing = PP_RESOLUTION_MISSING,
  ambiguous = PP_RESOLUTION_AMBIGUOUS,
  error = PP_RESOLUTION_ERROR,
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

struct Resolution final {
  Uuid representation_id;
  Uuid resource_id;
  ResolutionState state;
  std::vector<ResolutionCandidate> candidates;
  std::vector<Evidence> evidence;
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

inline Evidence resolution_evidence(const pp_resolution_set_t *resolutions,
                                    std::uint64_t resolution_index,
                                    std::uint64_t evidence_index) {
  pp_evidence_kind_t kind = 0;
  const char *detail = nullptr;
  pp_error_t *error = nullptr;
  const pp_error_code_t status = pp_resolution_evidence_get(
      resolutions, resolution_index, evidence_index, &kind, &detail, &error);
  throw_if_error(status, error);
  return {static_cast<EvidenceKind>(kind),
          detail != nullptr
              ? std::optional<std::string>(std::string(detail))
              : std::nullopt};
}

inline Evidence candidate_evidence(const pp_resolution_set_t *resolutions,
                                   std::uint64_t resolution_index,
                                   std::uint64_t candidate_index,
                                   std::uint64_t evidence_index) {
  pp_evidence_kind_t kind = 0;
  const char *detail = nullptr;
  pp_error_t *error = nullptr;
  const pp_error_code_t status = pp_resolution_candidate_evidence_get(
      resolutions, resolution_index, candidate_index, evidence_index, &kind,
      &detail, &error);
  throw_if_error(status, error);
  return {static_cast<EvidenceKind>(kind),
          detail != nullptr
              ? std::optional<std::string>(std::string(detail))
              : std::nullopt};
}

} // namespace detail

class Transaction final {
public:
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
  friend class Project;

  explicit Transaction(pp_transaction_t *transaction) noexcept
      : transaction_(transaction) {}

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

class Project final {
public:
  static Project create(std::string_view path) {
    return create_impl(path, nullptr);
  }

  static Project create(std::string_view path, std::string_view display_name) {
    const std::string name =
        detail::checked_string(display_name, "display_name");
    return create_impl(path, name.c_str());
  }

  static Project open(std::string_view path) {
    const std::string native_path = detail::checked_string(path, "path");
    pp_project_t *project = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status =
        pp_project_open(native_path.c_str(), &project, &error);
    detail::throw_if_error(status, error);
    return Project(project);
  }

  Project(const Project &) = delete;
  Project &operator=(const Project &) = delete;

  Project(Project &&other) noexcept
      : project_(std::exchange(other.project_, nullptr)) {}

  Project &operator=(Project &&other) noexcept {
    if (this != &other) {
      pp_project_release(project_);
      project_ = std::exchange(other.project_, nullptr);
    }
    return *this;
  }

  ~Project() { pp_project_release(project_); }

  [[nodiscard]] Uuid id() const {
    pp_uuid_t value{};
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_project_id(project_, &value, &error);
    detail::throw_if_error(status, error);

    return detail::uuid(value);
  }

  [[nodiscard]] bool containsAsset(const Uuid &asset_id) const {
    const pp_uuid_t value = detail::native_uuid(asset_id);
    std::uint8_t exists = 0;
    pp_error_t *error = nullptr;
    const pp_error_code_t status =
        pp_project_asset_exists(project_, &value, &exists, &error);
    detail::throw_if_error(status, error);
    return exists != 0;
  }

  [[nodiscard]] std::vector<ExternalIdentifier>
  externalIdentifiers(const ObjectRef &target) const {
    const pp_object_ref_t native_target = detail::native_object_ref(target);
    pp_external_identifier_set_t *raw_identifiers = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_project_external_identifiers(
        project_, &native_target, &raw_identifiers, &error);
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
    const pp_error_code_t status = pp_project_find_by_external_identifier(
        project_, native_scheme.c_str(), native_value.c_str(), &raw_objects,
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

  [[nodiscard]] std::vector<Resolution>
  resolveAsset(const Uuid &asset_id) const {
    const pp_uuid_t value = detail::native_uuid(asset_id);
    pp_resolution_set_t *raw_resolutions = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status = pp_project_resolve_asset(
        project_, &value, &raw_resolutions, &error);
    detail::throw_if_error(status, error);
    detail::ResolutionSetHandle resolutions(raw_resolutions);

    std::vector<Resolution> result;
    const std::uint64_t count = pp_resolution_set_count(resolutions.get());
    for (std::uint64_t resolution_index = 0; resolution_index < count;
         ++resolution_index) {
      pp_uuid_t representation_id{};
      pp_uuid_t resource_id{};
      pp_resolution_state_t state = 0;
      std::uint64_t candidate_count = 0;
      std::uint64_t evidence_count = 0;
      pp_error_t *item_error = nullptr;
      const pp_error_code_t item_status = pp_resolution_set_get(
          resolutions.get(), resolution_index, &representation_id, &resource_id,
          &state, &candidate_count, &evidence_count, &item_error);
      detail::throw_if_error(item_status, item_error);

      std::vector<ResolutionCandidate> candidates;
      for (std::uint64_t candidate_index = 0;
           candidate_index < candidate_count; ++candidate_index) {
        const char *uri = nullptr;
        std::uint16_t confidence = 0;
        std::uint64_t candidate_evidence_count = 0;
        pp_error_t *candidate_error = nullptr;
        const pp_error_code_t candidate_status = pp_resolution_candidate_get(
            resolutions.get(), resolution_index, candidate_index, &uri,
            &confidence, &candidate_evidence_count, &candidate_error);
        detail::throw_if_error(candidate_status, candidate_error);

        std::vector<Evidence> evidence;
        for (std::uint64_t evidence_index = 0;
             evidence_index < candidate_evidence_count; ++evidence_index) {
          evidence.push_back(detail::candidate_evidence(
              resolutions.get(), resolution_index, candidate_index,
              evidence_index));
        }
        candidates.push_back(
            {uri != nullptr ? std::string(uri) : std::string(), confidence,
             std::move(evidence)});
      }

      std::vector<Evidence> evidence;
      for (std::uint64_t evidence_index = 0;
           evidence_index < evidence_count; ++evidence_index) {
        evidence.push_back(detail::resolution_evidence(
            resolutions.get(), resolution_index, evidence_index));
      }
      result.push_back({detail::uuid(representation_id),
                        detail::uuid(resource_id),
                        static_cast<ResolutionState>(state),
                        std::move(candidates), std::move(evidence)});
    }
    return result;
  }

  [[nodiscard]] Transaction beginTransaction() {
    pp_transaction_t *transaction = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status =
        pp_project_begin_transaction(project_, &transaction, &error);
    detail::throw_if_error(status, error);
    return Transaction(transaction);
  }

  [[nodiscard]] explicit operator bool() const noexcept {
    return project_ != nullptr;
  }

private:
  explicit Project(pp_project_t *project) noexcept : project_(project) {}

  static Project create_impl(std::string_view path, const char *display_name) {
    const std::string native_path = detail::checked_string(path, "path");
    pp_project_t *project = nullptr;
    pp_error_t *error = nullptr;
    const pp_error_code_t status =
        pp_project_create(native_path.c_str(), display_name, &project, &error);
    detail::throw_if_error(status, error);
    return Project(project);
  }

  pp_project_t *project_ = nullptr;
};

[[nodiscard]] inline std::uint32_t abi_version() noexcept {
  return pp_abi_version();
}

} // namespace postproject

#endif
