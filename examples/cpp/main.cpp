#include <postproject/postproject.hpp>

#include <exception>
#include <iostream>

int main(int argc, char **argv) {
  if (argc != 3) {
    std::cerr
        << "usage: postproject-cpp-example OUTPUT_PRODUCTION MEDIA_FILE\n";
    return 2;
  }

  try {
    auto production = postproject::Production::create(argv[1], "C++ quickstart");
    auto transaction = production.beginTransaction();
    const auto asset_id = transaction.importMedia(argv[2], "Quickstart media");
    transaction.commit();
    std::cout << "representations: "
              << production.representations(asset_id).size() << '\n';
  } catch (const postproject::Error &error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
