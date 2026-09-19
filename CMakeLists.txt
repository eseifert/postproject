cmake_minimum_required(VERSION 3.21)

project(libpostproject VERSION 0.1.0 LANGUAGES NONE)

set(POSTPROJECT_PACKAGE_VERSION "0.1.0-alpha.1")

include(CMakePackageConfigHelpers)
include(GNUInstallDirs)

set(POSTPROJECT_LIBRARY "" CACHE FILEPATH "Path to the prebuilt libpostproject native library")
set(POSTPROJECT_STATIC_LIBRARY "" CACHE FILEPATH "Optional path to the prebuilt static library")
if(NOT POSTPROJECT_LIBRARY)
    message(FATAL_ERROR "POSTPROJECT_LIBRARY must name a library built by cargo build --release -p postproject-ffi")
endif()
if(NOT EXISTS "${POSTPROJECT_LIBRARY}")
    message(FATAL_ERROR "POSTPROJECT_LIBRARY does not exist: ${POSTPROJECT_LIBRARY}")
endif()

install(
    FILES
        include/postproject/postproject.h
        include/postproject/postproject.hpp
    DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}/postproject"
)
install(FILES "${POSTPROJECT_LIBRARY}" DESTINATION "${CMAKE_INSTALL_LIBDIR}")
if(POSTPROJECT_STATIC_LIBRARY)
    if(NOT EXISTS "${POSTPROJECT_STATIC_LIBRARY}")
        message(FATAL_ERROR "POSTPROJECT_STATIC_LIBRARY does not exist: ${POSTPROJECT_STATIC_LIBRARY}")
    endif()
    install(FILES "${POSTPROJECT_STATIC_LIBRARY}" DESTINATION "${CMAKE_INSTALL_LIBDIR}")
endif()
install(
    FILES LICENSE LICENSES/Apache-2.0.txt LICENSES/MIT.txt README.md CHANGELOG.md
    DESTINATION "${CMAKE_INSTALL_DATADIR}/doc/libpostproject"
)

configure_package_config_file(
    cmake/PostProjectConfig.cmake.in
    "${CMAKE_CURRENT_BINARY_DIR}/PostProjectConfig.cmake"
    INSTALL_DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/PostProject"
    PATH_VARS CMAKE_INSTALL_INCLUDEDIR CMAKE_INSTALL_LIBDIR
)
write_basic_package_version_file(
    "${CMAKE_CURRENT_BINARY_DIR}/PostProjectConfigVersion.cmake"
    VERSION "${PROJECT_VERSION}"
    COMPATIBILITY SameMajorVersion
)
configure_file(
    pkgconfig/postproject.pc.in
    "${CMAKE_CURRENT_BINARY_DIR}/postproject.pc"
    @ONLY
)

install(
    FILES
        "${CMAKE_CURRENT_BINARY_DIR}/PostProjectConfig.cmake"
        "${CMAKE_CURRENT_BINARY_DIR}/PostProjectConfigVersion.cmake"
    DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/PostProject"
)
install(
    FILES "${CMAKE_CURRENT_BINARY_DIR}/postproject.pc"
    DESTINATION "${CMAKE_INSTALL_LIBDIR}/pkgconfig"
)
