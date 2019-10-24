cmake_minimum_required(VERSION 3.13)
file(STRINGS "VERSION" OCKAM_VERSION)
project(ockam LANGUAGES C VERSION ${OCKAM_VERSION})

if (NOT CMAKE_ARCHIVE_OUTPUT_DIRECTORY)
    set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY "${PROJECT_BINARY_DIR}/lib")
endif()

# Ockam Common 
add_subdirectory("ockam/common/" "ockam/common/")

# Ockam Vault 
add_subdirectory("ockam/vault/" "ockam/vault/")

# Third Party Libraries
add_subdirectory("ockam/third-party/microchip" "ockam/third-party/microchip/")

# Ockam Tests
add_subdirectory("test/" "test/")
