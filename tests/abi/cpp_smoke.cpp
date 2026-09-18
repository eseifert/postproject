#include <postproject/postproject.hpp>

#include <cstdint>
#include <cstdio>
#include <exception>
#include <string>
#include <utility>

int main(int argc, char **argv) {
  if (argc != 2) {
    return 2;
  }

  const std::string path(argv[1]);
  std::remove(path.c_str());

  try {
    if (postproject::abi_version() != 1) {
      return 3;
    }

    auto project = postproject::Project::create(path, "C++ smoke test");
    const auto created_id = project.id();
    auto moved = std::move(project);
    if (project || !moved) {
      return 4;
    }

    auto reopened = postproject::Project::open(path);
    if (reopened.id() != created_id) {
      return 5;
    }

    try {
      static_cast<void>(postproject::Project::open(path + ".missing"));
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
