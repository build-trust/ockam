cmake_minimum_required(VERSION 3.14)

include(CMakePrintHelpers)

# Set some configs
set(OCKAM_C_ROOT_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(DEFAULT_CMAKE_BUILD_TYPE "Release")
set(CMAKE_POSITION_INDEPENDENT_CODE TRUE)
set(CMAKE_VERBOSE_MAKEFILE TRUE)

# Set the build type if one was not specified
if (NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
  message(STATUS "No build type selected, default to ${DEFAULT_CMAKE_BUILD_TYPE}")
  set(CMAKE_BUILD_TYPE "${DEFAULT_CMAKE_BUILD_TYPE}")
  message(STATUS "FORCING BUILD: '${CMAKE_BUILD_TYPE}'")
endif()

message(STATUS "OCKAM_C_ROOT_DIR               : '${OCKAM_C_ROOT_DIR}'")
message(STATUS "CMAKE_BUILD_TYPE               : '${CMAKE_BUILD_TYPE}'")

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
project(ockam
    VERSION 0.5
)

get_target_triple(OCKAM_TARGET_TRIPLE OCKAM_TARGET_ARCHITECTURE OCKAM_TARGET_VENDOR OCKAM_TARGET_OS OCKAM_TARGET_ABI)
get_host_triple(OCKAM_HOST_TRIPLE OCKAM_HOST_ARCHITECTURE OCKAM_HOST_VENDOR OCKAM_HOST_OS OCKAM_HOST_ABI)

# If the install prefix wasn't set, place the output in the build directory
set(CMAKE_INSTALL_PREFIX ${CMAKE_BINARY_DIR} CACHE PATH "Force install prefix" FORCE)

#TODO Clean-up install
#if((NOT DEFINED CMAKE_INSTALL_PREFIX) OR CMAKE_INSTALL_PREFIX_INITIALIZED_TO_DEFAULT)
#  set(CMAKE_INSTALL_PREFIX ${CMAKE_BINARY_DIR} CACHE PATH "Force install prefix" FORCE)
#endif()

include(ockam_macros)
include(ockam_copts)
include(ockam_cc_binary)
include(ockam_cc_library)
include(ockam_cc_test)
include(ockam_cc_alwayslink)
include(ockam_debug)
include(ockam_third_party)

# Set output directories
set(CMAKE_ARCHIVE_OUTPUT_DIRECTORY ${CMAKE_INSTALL_PREFIX}/${CMAKE_BUILD_TYPE}/lib)
set(CMAKE_LIBRARY_OUTPUT_DIRECTORY ${CMAKE_INSTALL_PREFIX}/${CMAKE_BUILD_TYPE}/lib)
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_INSTALL_PREFIX}/${CMAKE_BUILD_TYPE}/bin)
set(OCKAM_INCLUDES_OUTPUT_DIRECTORY ${CMAKE_INSTALL_PREFIX}/${CMAKE_BUILD_TYPE}/includes)
set(OCKAM_TESTS_OUTPUT_DIRECTORY ${CMAKE_INSTALL_PREFIX}/${CMAKE_BUILD_TYPE}/tests)
set(OCKAM_FIXTURES_OUTPUT_DIRECTORY ${OCKAM_TESTS_OUTPUT_DIRECTORY}/fixtures)

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

message(STATUS "CMake Version                  : " "${CMAKE_SYSTEM_VERSION}")
message(STATUS "Compiler                       : " "${CMAKE_C_COMPILER}")
message(STATUS "Target                         : " "${CMAKE_C_COMPILER_TARGET}")
message(STATUS "Operating System               : " "${CMAKE_SYSTEM}")
message(STATUS "Build Tests                    : " "${OCKAM_BUILD_TESTS}")

string(JOIN " " CMAKE_CXX_FLAGS ${OCKAM_DEFAULT_COPTS})

if(${OCKAM_BUILD_TESTS})
  enable_testing(lib)
endif()

set_alwayslink_ockam_libs()
add_subdirectory(lib)
ockam_complete_binary_link_options()
