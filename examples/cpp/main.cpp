#include <postproject/postproject.hpp>

#include <exception>
#include <iostream>

int main(int argc, char **argv) {
  if (argc != 2) {
    std::cerr << "usage: postproject-cpp-example PRODUCTION\n";
    return 2;
  }

  try {
    const auto production = postproject::Production::open(argv[1]);
    const auto id = production.id();
    std::cout << "opened production with " << id.bytes().size()
              << " identity bytes\n";
  } catch (const postproject::Error &error) {
    std::cerr << error.what() << '\n';
    return 1;
  }
}
