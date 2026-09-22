cmake_minimum_required(VERSION 3.21)

project(postproject_cpp_example LANGUAGES CXX)

find_package(PostProject 0.2 REQUIRED CONFIG)

add_executable(postproject-cpp-example main.cpp)
target_compile_features(postproject-cpp-example PRIVATE cxx_std_17)
target_link_libraries(postproject-cpp-example PRIVATE PostProject::postproject)
