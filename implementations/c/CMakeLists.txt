cmake_minimum_required(VERSION 3.14)
file(STRINGS "VERSION" OCKAM_VERSION)
project(ockam LANGUAGES C VERSION ${OCKAM_VERSION})

set(CMAKE_VERBOSE_MAKEFILE TRUE)

# Ockam Library
add_subdirectory("ockam/vault/")

# Third Party Libraries
add_subdirectory("ockam/third-party/microchip")

# Ockam Tests
add_subdirectory("ockam/test/")

#install(TARGETS ockam_vault DESTINATION lib)

#install(DIRECTORY include/ DESTINATION include/
#        FILES_MATCHING PATTERN "*.h")
