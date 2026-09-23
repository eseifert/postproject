cmake_minimum_required(VERSION 3.21)

# Builds and runs every non-Rust listing included in the guides. Configure it
# only against an installed PostProject prefix; it never invokes Cargo. The
# Rust listings run as the postproject-doc-examples workspace test.
project(postproject_doc_examples LANGUAGES C CXX)

find_package(PostProject 0.3 REQUIRED CONFIG)
find_package(Python3 3.11 COMPONENTS Interpreter)
find_program(POSTPROJECT_EXECUTABLE postproject)
find_program(BASH_EXECUTABLE bash)
find_program(JQ_EXECUTABLE jq)

set(POSTPROJECT_MEDIA_FIXTURE
    "${CMAKE_CURRENT_SOURCE_DIR}/../../examples/fixtures/sample-media.dat"
    CACHE FILEPATH "Media file imported by the guide examples")
set(POSTPROJECT_PYTHON_SOURCE
    "${CMAKE_CURRENT_SOURCE_DIR}/../../python/src"
    CACHE PATH "Directory containing the postproject Python package")

add_executable(postproject-c-guides c/guides.c)
target_compile_features(postproject-c-guides PRIVATE c_std_11)
target_link_libraries(postproject-c-guides PRIVATE PostProject::postproject)

add_executable(postproject-cpp-guides cpp/guides.cpp)
target_compile_features(postproject-cpp-guides PRIVATE cxx_std_17)
target_link_libraries(postproject-cpp-guides PRIVATE PostProject::postproject)

foreach(example IN ITEMS postproject-c-guides postproject-cpp-guides)
    if(MSVC)
        target_compile_options("${example}" PRIVATE /W4 /WX)
    else()
        target_compile_options("${example}" PRIVATE -Wall -Wextra -Wpedantic -Werror)
    endif()
    if(WIN32)
        add_custom_command(
            TARGET "${example}"
            POST_BUILD
            COMMAND "${CMAKE_COMMAND}" -E copy_if_different
                "$<TARGET_FILE:PostProject::postproject>"
                "$<TARGET_FILE_DIR:${example}>"
        )
    endif()
endforeach()
if(MSVC)
    target_compile_definitions(postproject-c-guides PRIVATE _CRT_SECURE_NO_WARNINGS)
endif()

enable_testing()

# Each surface gets a fresh work directory prepared by a setup fixture.
function(add_guide_test surface)
    set(work_dir "${CMAKE_CURRENT_BINARY_DIR}/work-${surface}")
    add_test(
        NAME "${surface}-guides-prepare"
        COMMAND "${CMAKE_COMMAND}"
            "-DWORK_DIR=${work_dir}"
            "-DMEDIA_FIXTURE=${POSTPROJECT_MEDIA_FIXTURE}"
            -P "${CMAKE_CURRENT_SOURCE_DIR}/prepare-workdir.cmake"
    )
    set_tests_properties("${surface}-guides-prepare" PROPERTIES FIXTURES_SETUP "${surface}-work")
    add_test(NAME "${surface}-guides" COMMAND ${ARGN} "${work_dir}")
    set_tests_properties("${surface}-guides" PROPERTIES FIXTURES_REQUIRED "${surface}-work")
endfunction()

add_guide_test(c "$<TARGET_FILE:postproject-c-guides>")
add_guide_test(cpp "$<TARGET_FILE:postproject-cpp-guides>")

if(Python3_FOUND)
    add_guide_test(python "${Python3_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/python/guides.py")
    set_tests_properties(
        python-guides
        PROPERTIES
            ENVIRONMENT_MODIFICATION
                "PYTHONPATH=path_list_prepend:${POSTPROJECT_PYTHON_SOURCE};POSTPROJECT_LIBRARY=set:$<TARGET_FILE:PostProject::postproject>"
    )
else()
    message(WARNING "Python 3.11 or newer not found; skipping the Python guide examples")
endif()

if(NOT WIN32 AND POSTPROJECT_EXECUTABLE AND BASH_EXECUTABLE AND JQ_EXECUTABLE)
    get_filename_component(postproject_bin "${POSTPROJECT_EXECUTABLE}" DIRECTORY)
    add_guide_test(cli "${BASH_EXECUTABLE}" "${CMAKE_CURRENT_SOURCE_DIR}/cli/guides.sh")
    set_tests_properties(
        cli-guides
        PROPERTIES ENVIRONMENT_MODIFICATION "PATH=path_list_prepend:${postproject_bin}"
    )
else()
    message(WARNING "postproject, bash, or jq not found; skipping the CLI guide examples")
endif()
