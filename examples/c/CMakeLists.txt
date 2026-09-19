cmake_minimum_required(VERSION 3.21)

project(postproject_c_example LANGUAGES C)

find_package(PostProject 0.1 REQUIRED CONFIG)

add_executable(postproject-c-example main.c)
target_compile_features(postproject-c-example PRIVATE c_std_11)
target_link_libraries(postproject-c-example PRIVATE PostProject::postproject)
