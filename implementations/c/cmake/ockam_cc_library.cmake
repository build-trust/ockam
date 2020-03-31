include(CMakeParseArguments)

# ockam_cc_library()
#
# CMake function to construct a library target
#
# Parameters:
# NAME: name of target (see Note)
# HDRS: List of public header files for the library
# TEXTUAL_HDRS: List of public header files that cannot be compiled on their own
# SRCS: List of source files for the library
# DEPS: List of other libraries to be linked in to the binary targets
# COPTS: List of private compile options
# DEFINES: List of public defines
# INCLUDES: Include directories to add to dependencies
# LINKOPTS: List of link options
# ALWAYSLINK: Always link the library into any binary with a direct dep.
# PUBLIC: Add this so that this library will be exported under ockam::
# Also in IDE, target will appear in Ockam folder while non PUBLIC will be in Ockam/internal.
# TESTONLY: When added, this target will only be built if user passes -DOCKAM_BUILD_TESTS=ON to CMake.
#
# Note:
# By default, ockam_cc_library will always create a library named ockam_${NAME},
# and alias target ockam::${NAME}. The ockam:: form should always be used.
# This is to reduce namespace pollution.
#
# ockam_cc_library(
#   NAME
#     foo
#   HDRS
#     "foo.h"
#   SRCS
#     "foo.cc"
# )
# ockam_cc_library(
#   NAME
#     bar
#   SRCS
#     "bar.cc"
#   DEPS
#     ockam::bar
#   PUBLIC
# )
#
# ockam_cc_library(
#   NAME
#     main_lib
#   ...
#   DEPS
#     ockam::bar
# )
function(ockam_cc_library)
  cmake_parse_arguments(
    _RULE
    "PUBLIC;ALWAYSLINK;TESTONLY;SHARED"
    "NAME"
    "HDRS;TEXTUAL_HDRS;SRCS;COPTS;DEFINES;LINKOPTS;DEPS;INCLUDES"
    ${ARGN}
  )

  message(STATUS "---------------- ockam_cc_library ----------------")
  message(STATUS "_RULE_NAME                     : '${_RULE_NAME}'")
  ockam_package_ns(_PACKAGE_NS)

  # Replace dependencies passed by ::name with ::ockam::name
  list(TRANSFORM _RULE_DEPS REPLACE "^::" "${_PACKAGE_NS}::")
  set(_NAME "_ockam_${_RULE_NAME}")
  message(STATUS "_NAME                          : '${_NAME}'")
  message(STATUS "_RULE_INCLUDES                 : '${_RULE_INCLUDES}'")

  if(NOT _RULE_TESTONLY OR OCKAM_BUILD_TESTS)

    # Prefix the library with the package name, so we get: ockam_package_name.
    ockam_package_name(_PACKAGE_NAME)
    set(_NAME "${_PACKAGE_NAME}")
    message(STATUS "_NAME                          : '${_NAME}'")

    set(_CC_SRCS "${_RULE_SRCS}")
    foreach(src_file IN LISTS _CC_SRCS)
      if(${src_file} MATCHES ".*\\.(h|inc)")
        list(REMOVE_ITEM _CC_SRCS "${src_file}")
      endif()
    endforeach()
    if("${_CC_SRCS}" STREQUAL "")
      set(_RULE_IS_INTERFACE 1)
    else()
      set(_RULE_IS_INTERFACE 0)
    endif()

    if(NOT _RULE_IS_INTERFACE)
      if (NOT _RULE_SHARED)
        message(STATUS "add_library                    : '${_NAME}'")
        add_library(${_NAME} STATIC "")
      else()
        add_library(${_NAME} STATIC SHARED "")
      endif()
      target_sources(${_NAME}
        PRIVATE
          ${_RULE_SRCS}
          ${_RULE_TEXTUAL_HDRS}
          ${_RULE_HDRS}
      )
      target_include_directories(${_NAME}
        PUBLIC
          "$<BUILD_INTERFACE:${OCKAM_COMMON_INCLUDE_DIRS}>"
          "$<BUILD_INTERFACE:${_RULE_INCLUDES}>"
      )
      target_compile_options(${_NAME}
        PRIVATE
          ${_RULE_COPTS}
          ${OCKAM_DEFAULT_COPTS}
      )

      target_link_libraries(${_NAME}
        PUBLIC
          ${_RULE_DEPS}
        PRIVATE
          ${_RULE_LINKOPTS}
          ${OCKAM_DEFAULT_LINKOPTS}
      )
      target_compile_definitions(${_NAME}
        PUBLIC
          ${_RULE_DEFINES}
      )

      if(DEFINED _RULE_ALWAYSLINK)
        set_property(TARGET ${_NAME} PROPERTY ALWAYSLINK 1)
      endif()

      # Add all Ockam targets to a folder in the IDE for organization.
      if(_RULE_PUBLIC)
        set_property(TARGET ${_NAME} PROPERTY FOLDER ${OCKAM_IDE_FOLDER})
      elseif(_RULE_TESTONLY)
        set_property(TARGET ${_NAME} PROPERTY FOLDER ${OCKAM_IDE_FOLDER}/tests)
      else()
        set_property(TARGET ${_NAME} PROPERTY FOLDER ${OCKAM_IDE_FOLDER}/internal)
      endif()

      # INTERFACE libraries can't have the C_STANDARD/CXX_STANDARD property set.
      set_property(TARGET ${_NAME} PROPERTY CXX_STANDARD ${OCKAM_CXX_STANDARD})
      set_property(TARGET ${_NAME} PROPERTY CXX_STANDARD_REQUIRED ON)
      set_property(TARGET ${_NAME} PROPERTY C_STANDARD ${OCKAM_C_STANDARD})
      set_property(TARGET ${_NAME} PROPERTY C_STANDARD_REQUIRED ON)
    else()
      # Generating header-only library.
      add_library(${_NAME} INTERFACE)
      target_include_directories(${_NAME}
        INTERFACE
        "$<BUILD_INTERFACE:${OCKAM_COMMON_INCLUDE_DIRS}>"
      )
      target_compile_options(${_NAME}
        INTERFACE
          ${_RULE_COPTS}
          ${OCKAM_DEFAULT_COPTS}
      )
      target_link_libraries(${_NAME}
        INTERFACE
          ${_RULE_DEPS}
          ${_RULE_LINKOPTS}
          ${OCKAM_DEFAULT_LINKOPTS}
      )
      target_compile_definitions(${_NAME}
        INTERFACE
          ${_RULE_DEFINES}
      )
    endif()

    install(TARGETS ${_NAME}
      ARCHIVE DESTINATION ${CMAKE_ARCHIVE_OUTPUT_DIRECTORY}
      LIBRARY DESTINATION ${CMAKE_LIBRARY_OUTPUT_DIRECTORY}
    )

    message(STATUS "CMAKE_ARCHIVE_OUTPUT_DIRECTORY : '${CMAKE_ARCHIVE_OUTPUT_DIRECTORY}'")
    message(STATUS "CMAKE_LIBRARY_OUTPUT_DIRECTORY : '${CMAKE_LIBRARY_OUTPUT_DIRECTORY}'")

    # Alias the ockam_package_name library to ockam::package_name.
    # This makes it possible to disambiguate the underscores in paths vs. the separators.
    add_library(${_PACKAGE_NS}::${_RULE_NAME} ALIAS ${_NAME})
    ockam_package_dir(_PACKAGE_DIR)
    message(STATUS "_PACKAGE_DIR                   : '${_PACKAGE_DIR}'")
    if(${_RULE_NAME} STREQUAL ${_PACKAGE_DIR})
      # If the library name matches the package then treat it as a default.
      # For example, foo/bar/ library 'bar' would end up as 'foo::bar'.
      add_library(${_PACKAGE_NS} ALIAS ${_NAME})
    endif()
  endif()
endfunction()
