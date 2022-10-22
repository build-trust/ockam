
# Set the minimum required CMake version to 3.1...3.18
#
# * We require at least CMake 3.1 (which added the target_sources command)
# * has been tested for versions up to 3.18
#
# The FATAL_ERROR option is accepted, but ignored, by CMake 2.6 and higher.
# It's specified so CMake versions 2.4 and lower fail with an error instead of
# just a warning.
#
# Release Notes by CMake Version
# cmake.org/cmake/help/latest/release/index.html
#
# CMake Policies by CMake Version
# cmake.org/cmake/help/latest/manual/cmake-policies.7.html
cmake_minimum_required(VERSION 3.1...3.18 FATAL_ERROR)

# Set PROJECT_NAME, PROJECT_VERSION and select which languages are needed to
# build this project.
#
# If no language is specified, cmake will enable C and CXX and test that it
# can find both compilers. We don't use the C++ compiler, so by sepecifying
# only C here we tell CMake to test only for a presence of the C compiler.
# This way the build environment isn't required to have a CXX compiler.
project(ockam VERSION 0.10.1 LANGUAGES C)

# If this is the top level CMake project.
if(CMAKE_PROJECT_NAME STREQUAL PROJECT_NAME)
  # Configure this project for testing with CTest/CDash
  #
  # Including CTest module enables testing and creates a BUILD_TESTING option.
  # The BUILD_TESTING option is ON by default.
  include(CTest)
endif()

# If BUILD_TESTING is ON, but this is not the top level project then someone
# has included our project into theirs and when they set BUILD_TESTING to ON
# they likely don't want our tests to build. So we turn BUILD_TESTING to OFF
# in our project scope.
#
# In the rare case that a higher level project really wants our tests to build.
# they must set OCKAM_BUILD_TESTING to ON in addition to setting BUILD_TESTING.
if(BUILD_TESTING)
  if(NOT (CMAKE_PROJECT_NAME STREQUAL PROJECT_NAME OR OCKAM_BUILD_TESTING))
    set(BUILD_TESTING OFF)
  endif()
endif()

# We need FetchContent to download external dependencies in the various
# subdirectories.
#
# FetchContent was added in CMake version 3.11, for versions lower than that
# we include a vendored copy of the module's code from CMake v3.11.3
if(CMAKE_VERSION VERSION_LESS 3.11)
  include("${CMAKE_CURRENT_SOURCE_DIR}/cmake/v3.11.3/FetchContent.cmake")
else()
  include(FetchContent)
endif()

option(OCKAM_ENABLE_ATECC608A_BUILD "Enables build for atecc608a vault"                                        ON )
option(OCKAM_ENABLE_ATECC608A_TESTS "Enables tests for atecc608a vault"                                        OFF)
option(OCKAM_DISABLE_LOG            "Disables logging (also reduces size of binary by cutting out log string)" OFF)
option(OCKAM_CUSTOM_LOG_FUNCTION    "Allows setting custom log function (default uses stdout)"                 OFF)

# add external dependencies
add_subdirectory(external)

# add ockam modules
add_subdirectory(ockam)
