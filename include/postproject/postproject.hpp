#ifndef POSTPROJECT_POSTPROJECT_HPP
#define POSTPROJECT_POSTPROJECT_HPP

#include <postproject/postproject.h>

#include <array>
#include <cstdint>
#include <memory>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>

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
    return left.bytes_ == right.bytes_;
  }

  friend constexpr bool operator!=(const Uuid &left,
                                   const Uuid &right) noexcept {
    return !(left == right);
  }

private:
  Bytes bytes_;
};

namespace detail {

struct ErrorDeleter final {
  void operator()(pp_error_t *error) const noexcept { pp_error_release(error); }
};

using ErrorHandle = std::unique_ptr<pp_error_t, ErrorDeleter>;

inline void throw_if_error(pp_error_code_t status, pp_error_t *raw_error) {
  ErrorHandle error(raw_error);
  if (status == PP_OK) {
    return;
  }

  const char *raw_message = error ? pp_error_message(error.get()) : nullptr;
  std::string message =
      raw_message != nullptr ? raw_message : "libpostproject operation failed";
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

} // namespace detail

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

    Uuid::Bytes bytes{};
    for (std::size_t index = 0; index < bytes.size(); ++index) {
      bytes[index] = value.bytes[index];
    }
    return Uuid(bytes);
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
