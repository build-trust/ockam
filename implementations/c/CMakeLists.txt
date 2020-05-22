cmake_minimum_required(VERSION 3.14)

include(CMakePrintHelpers)

# Set some configs
set(OCKAM_C_ROOT_DIR ${CMAKE_CURRENT_LIST_DIR})
set(DEFAULT_CMAKE_BUILD_TYPE "Release")
set(CMAKE_POSITION_INDEPENDENT_CODE TRUE)

# Set the build type if one was not specified
if (NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
  message(STATUS "No build type selected, default to ${DEFAULT_CMAKE_BUILD_TYPE}")
  set(CMAKE_BUILD_TYPE "${DEFAULT_CMAKE_BUILD_TYPE}")
  message(STATUS "FORCING BUILD: '${CMAKE_BUILD_TYPE}'")
endif()

message(STATUS "OCKAM_C_ROOT_DIR               : '${OCKAM_C_ROOT_DIR}'")
message(STATUS "CMAKE_BUILD_TYPE               : '${CMAKE_BUILD_TYPE}'")
message(STATUS "CMAKE_INSTALL_PREFIX           : '${CMAKE_INSTALL_PREFIX}'")

# We don't want builds stomping all over the source directory
if (CMAKE_SOURCE_DIR STREQUAL CMAKE_BINARY_DIR AND NOT MSVC_IDE)
  message(FATAL_ERROR "In-source builds are not allowed. "
    "Please create a directory and run cmake "
    "from there, passing the path to this source directory as the last argument. "
    "This process created the file `CMakeCache.txt' and the directory "
    "`CMakeFiles'. Please delete them.")
endif()

# Folder configuration
set(OCKAM_IDE_FOLDER Ockam)
set_property(GLOBAL PROPERTY USE_FOLDERS ON)

# Load helper modules
list(INSERT CMAKE_MODULE_PATH 0 "${CMAKE_CURRENT_SOURCE_DIR}/cmake")

# Grab target and host triples
include(ockam_triples)
include(ockam_platform)

# Define the project AFTER the platform has been set!
set(OCKAM_MAJOR_VERSION 0)
set(OCKAM_MINOR_VERSION 6)
set(OCKAM_VERSION ${OCKAM_MAJOR_VERSION}.${OCKAM_MINOR_VERSION})
project(ockam
  VERSION ${OCKAM_VERSION}
)

get_target_triple(OCKAM_TARGET_TRIPLE OCKAM_TARGET_ARCHITECTURE OCKAM_TARGET_VENDOR OCKAM_TARGET_OS OCKAM_TARGET_ABI)
get_host_triple(OCKAM_HOST_TRIPLE OCKAM_HOST_ARCHITECTURE OCKAM_HOST_VENDOR OCKAM_HOST_OS OCKAM_HOST_ABI)

# If the install prefix wasn't set, place the output in the build directory
if(CMAKE_INSTALL_PREFIX MATCHES "^C:/Program Files" OR CMAKE_INSTALL_PREFIX MATCHES "^/usr/local$")
  set(CMAKE_INSTALL_PREFIX ${CMAKE_BINARY_DIR})
else()
  set(CMAKE_INSTALL_PREFIX "${CMAKE_INSTALL_PREFIX}/${OCKAM_TARGET_TRIPLE}")
endif()

include(ockam_macros)
include(ockam_copts)
include(ockam_cc_binary)
include(ockam_cc_library)
include(ockam_cc_test)
include(ockam_cc_alwayslink)
include(ockam_debug)
include(ockam_third_party)

# Set output directories
set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/${CMAKE_BUILD_TYPE}/lib)
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/${CMAKE_BUILD_TYPE}/lib)
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/${CMAKE_BUILD_TYPE}/bin)
set(OCKAM_INCLUDES_OCKAM_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/${CMAKE_BUILD_TYPE}/include/ockam)
set(OCKAM_TESTS_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/${CMAKE_BUILD_TYPE}/tests)

# Global output directories
set(OCKAM_LIB_OUTPUT_DIRECTORY ${CMAKE_LIBRARY_OUTPUT_DIRECTORY} CACHE PATH "Ockam Include Force" FORCE)
set(OCKAM_INCLUDES_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/${CMAKE_BUILD_TYPE}/include CACHE PATH "Ockam Include Force" FORCE)

# Set cmake config options
set(OCKAM_CMAKE_INSTALL_DIRECTORY "${CMAKE_LIBRARY_OUTPUT_DIRECTORY}/CMake/ockam")
set(OCKAM_EXPORT_TARGETS_FILE "${OCKAM_CMAKE_INSTALL_DIRECTORY}/ockamTargets.cmake")

message(STATUS "OCKAM_TARGET_ARCHITECTURE      : '${OCKAM_TARGET_ARCHITECTURE}'")
message(STATUS "OCKAM_TARGET_VENDOR            : '${OCKAM_TARGET_VENDOR}'")
message(STATUS "OCKAM_TARGET_OS                : '${OCKAM_TARGET_OS}'")
message(STATUS "OCKAM_TARGET_ABI               : '${OCKAM_TARGET_ABI}'")
message(STATUS "OCKAM_TARGET_TRIPLE            : '${OCKAM_TARGET_TRIPLE}'")
message(STATUS "OCKAM_HOST_ARCHITECTURE        : '${OCKAM_HOST_ARCHITECTURE}'")
message(STATUS "OCKAM_HOST_VENDOR              : '${OCKAM_HOST_VENDOR}'")
message(STATUS "OCKAM_HOST_OS                  : '${OCKAM_HOST_OS}'")
message(STATUS "OCKAM_HOST_ABI                 : '${OCKAM_HOST_ABI}'")
message(STATUS "OCKAM_HOST_TRIPLE              : '${OCKAM_HOST_TRIPLE}'")

if (ENV{VERBOSE})
    set(CMAKE_INSTALL_MESSAGE ALWAYS)
else()
    set(CMAKE_INSTALL_MESSAGE LAZY)
endif()

option(OCKAM_BUILD_TESTS "Builds Ockam tests." OFF)

string(JOIN " " CMAKE_CXX_FLAGS ${OCKAM_DEFAULT_COPTS})
message(STATUS "CMake Version                  : " "${CMAKE_SYSTEM_VERSION}")
message(STATUS "Compiler                       : " "${CMAKE_C_COMPILER}")
message(STATUS "Target                         : " "${CMAKE_C_COMPILER_TARGET}")
message(STATUS "Operating System               : " "${CMAKE_SYSTEM}")
message(STATUS "Build Tests                    : " "${OCKAM_BUILD_TESTS}")
message(STATUS "CXX Flags                      : " "${CMAKE_CXX_FLAGS}")

if(${OCKAM_BUILD_TESTS})
  enable_testing(lib)
endif()

set_alwayslink_ockam_libs()
set_alwayslink_ockam_includes()
add_subdirectory(lib)
ockam_complete_binary_link_options()
