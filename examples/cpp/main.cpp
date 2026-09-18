#include <postproject/postproject.hpp>

#include <exception>
#include <iostream>

int main(int argc, char **argv) {
  if (argc != 2) {
    std::cerr << "usage: postproject-cpp-example PROJECT\n";
    return 2;
  }

  try {
    const auto project = postproject::Project::open(argv[1]);
    const auto id = project.id();
    std::cout << "opened project with " << id.bytes().size()
              << " identity bytes\n";
  } catch (const postproject::Error &error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
