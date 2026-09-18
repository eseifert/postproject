cmake_minimum_required(VERSION 3.21)

project(postproject_cpp_consumer LANGUAGES CXX)

find_package(PostProject 0.1 REQUIRED CONFIG)

add_executable(postproject-cpp-smoke cpp_smoke.cpp)
target_compile_features(postproject-cpp-smoke PRIVATE cxx_std_17)
target_link_libraries(postproject-cpp-smoke PRIVATE PostProject::postproject)

if(MSVC)
    target_compile_options(postproject-cpp-smoke PRIVATE /W4 /WX)
else()
    target_compile_options(postproject-cpp-smoke PRIVATE -Wall -Wextra -Wpedantic -Werror)
endif()

enable_testing()
add_test(NAME postproject-cpp-smoke COMMAND postproject-cpp-smoke "${CMAKE_CURRENT_BINARY_DIR}/cpp-smoke.pproj")
