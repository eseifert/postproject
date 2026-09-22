cmake_minimum_required(VERSION 3.21)

project(postproject_native_consumers LANGUAGES C CXX)

find_package(PostProject 0.2 REQUIRED CONFIG)

add_executable(postproject-cpp-smoke cpp_smoke.cpp)
target_compile_features(postproject-cpp-smoke PRIVATE cxx_std_17)
target_link_libraries(postproject-cpp-smoke PRIVATE PostProject::postproject)

add_executable(postproject-c-smoke c_smoke.c)
target_compile_features(postproject-c-smoke PRIVATE c_std_11)
target_link_libraries(postproject-c-smoke PRIVATE PostProject::postproject)

if(WIN32)
    foreach(consumer IN ITEMS postproject-c-smoke postproject-cpp-smoke)
        add_custom_command(
            TARGET "${consumer}"
            POST_BUILD
            COMMAND "${CMAKE_COMMAND}" -E copy_if_different
                "$<TARGET_FILE:PostProject::postproject>"
                "$<TARGET_FILE_DIR:${consumer}>"
        )
    endforeach()
endif()

if(MSVC)
    target_compile_options(postproject-cpp-smoke PRIVATE /W4 /WX)
    target_compile_options(postproject-c-smoke PRIVATE /W4 /WX)
    target_compile_definitions(postproject-c-smoke PRIVATE _CRT_SECURE_NO_WARNINGS)
else()
    target_compile_options(postproject-cpp-smoke PRIVATE -Wall -Wextra -Wpedantic -Werror)
    target_compile_options(postproject-c-smoke PRIVATE -Wall -Wextra -Wpedantic -Werror)
endif()

enable_testing()
add_test(NAME postproject-cpp-smoke COMMAND postproject-cpp-smoke "${CMAKE_CURRENT_BINARY_DIR}/cpp-smoke.pproj")
add_test(
    NAME postproject-c-smoke
    COMMAND postproject-c-smoke
        "${CMAKE_CURRENT_BINARY_DIR}/c-smoke.pproj"
        "${CMAKE_CURRENT_BINARY_DIR}"
)
