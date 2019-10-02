cmake_minimum_required(VERSION 3.13)
file(STRINGS "VERSION" OCKAM_VERSION)
project(ockam LANGUAGES C VERSION ${OCKAM_VERSION})

# Ockam Library
add_subdirectory("ockam/vault/")

# Third Party Libraries
add_subdirectory("ockam/third-party/microchip")

# Ockam Tests
add_subdirectory("ockam/test/")
