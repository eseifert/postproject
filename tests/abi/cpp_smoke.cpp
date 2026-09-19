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
    if (postproject::abi_version() != 1) {
      return 3;
    }

    auto project = postproject::Project::create(path, "C++ smoke test");
    const auto created_id = project.id();
    const std::string media_path = path + ".media";
    {
      std::ofstream media(media_path, std::ios::binary);
      media << "C++ transaction media";
      if (!media) {
        return 10;
      }
    }
    auto transaction = project.beginTransaction();
    const auto asset_id = transaction.importMedia(media_path, "C++ asset");
    static_cast<void>(transaction.addMediaRoot(
        std::filesystem::path(path).parent_path().string(), "fixtures"));
    transaction.commit();
    if (!project.containsAsset(asset_id)) {
      return 8;
    }

    auto rolled_back = project.beginTransaction();
    const auto discarded_id = rolled_back.importMedia(media_path);
    rolled_back.rollback();
    if (project.containsAsset(discarded_id)) {
      return 9;
    }

    auto moved = std::move(project);
    if (project || !moved) {
      return 4;
    }

    auto reopened = postproject::Project::open(path);
    if (reopened.id() != created_id || !reopened.containsAsset(asset_id)) {
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
