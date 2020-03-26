include(CMakeParseArguments)

if (NOT DEFINED _OCKAM_CC_BINARY_NAMES)
  set(_OCKAM_CC_BINARY_NAMES "")
endif()

# ockam_cc_binary()
#
# Parameters:
# NAME: name of target (see Usage below)
# SRCS: List of source files for the binary
# DEPS: List of other libraries to be linked in to the binary targets
# COPTS: List of private compile options
# DEFINES: List of public defines
# LINKOPTS: List of link options
#
# Note:
# By default, ockam_cc_binary will always create a binary named ockam_${NAME}.
#
# Usage:
# ockam_cc_library(
#   NAME
#     foo
#   HDRS
#     "foo.h"
#   SRCS
#     "foo.cc"
#   PUBLIC
# )
#
# ockam_cc_binary(
#   NAME
#     bar_tool
#   OUT
#     bar-tool
#   SRCS
#     "bar_tool_main.cc"
#   DEPS
#     ockam::foo
# )
function(ockam_cc_binary)
  cmake_parse_arguments(
    _RULE
    ""
    "NAME;OUT"
    "SRCS;COPTS;DEFINES;LINKOPTS;DEPS"
    ${ARGN}
  )

  message(STATUS "---------------- ockam_cc_binary -----------------")

  # Prefix the library with the package name, so we get: ockam_package_name
  ockam_package_name(_PACKAGE_NAME)
  #set(_NAME "${_PACKAGE_NAME}_${_RULE_NAME}")
  set(_NAME "${_RULE_NAME}")
  message(STATUS "_NAME                          : '${_NAME}'")

  add_executable(${_NAME} "")
  if(_RULE_SRCS)
    target_sources(${_NAME}
      PRIVATE
        ${_RULE_SRCS}
    )
  else()
    set(_DUMMY_SRC "${CMAKE_CURRENT_BINARY_DIR}/${_NAME}_dummy.cc")
    file(WRITE ${_DUMMY_SRC} "")
    target_sources(${_NAME}
      PRIVATE
        ${_DUMMY_SRC}
    )
  endif()
  if(_RULE_OUT)
    set_target_properties(${_NAME} PROPERTIES OUTPUT_NAME "${_RULE_OUT}")
  endif()
  target_include_directories(${_NAME}
    PUBLIC
      ${OCKAM_COMMON_INCLUDE_DIRS}
  )
  target_compile_definitions(${_NAME}
    PUBLIC
      ${_RULE_DEFINES}
  )
  target_compile_options(${_NAME}
    PRIVATE
      ${_RULE_COPTS}
  )

  ockam_package_ns(_PACKAGE_NS)

  # Replace dependencies passed by ::name with ::ockam::package::name
  list(TRANSFORM _RULE_DEPS REPLACE "^::" "${_PACKAGE_NS}::")

  # Add all OCKAM targets to a folder in the IDE for organization.
  set_property(TARGET ${_NAME} PROPERTY FOLDER ${OCKAM_IDE_FOLDER}/binaries)

  set_property(TARGET ${_NAME} PROPERTY CXX_STANDARD ${OCKAM_CXX_STANDARD})
  set_property(TARGET ${_NAME} PROPERTY CXX_STANDARD_REQUIRED ON)
  set_property(TARGET ${_NAME} PROPERTY C_STANDARD ${OCKAM_C_STANDARD})
  set_property(TARGET ${_NAME} PROPERTY C_STANDARD_REQUIRED ON)

  # Defer computing transitive dependencies and calling target_link_libraries()
  # until all libraries have been declared.
  # Track target and deps, use in ockam_complete_binary_link_options() later.
  set_property(GLOBAL APPEND PROPERTY _OCKAM_CC_BINARY_NAMES "${_NAME}")
  set_property(TARGET ${_NAME} PROPERTY DIRECT_DEPS ${_RULE_DEPS})
endfunction()

# Lists all transitive dependencies of DIRECT_DEPS in TRANSITIVE_DEPS.
function(_ockam_transitive_dependencies DIRECT_DEPS TRANSITIVE_DEPS)
  set(_TRANSITIVE "")

  foreach(_DEP ${DIRECT_DEPS})
    _ockam_transitive_dependencies_helper(${_DEP} _TRANSITIVE)
  endforeach(_DEP)

  set(${TRANSITIVE_DEPS} "${_TRANSITIVE}" PARENT_SCOPE)
endfunction()

# Recursive helper function for _ockam_transitive_dependencies.
# Performs a depth-first search through the dependency graph, appending all
# dependencies of TARGET to the TRANSITIVE_DEPS list.
function(_ockam_transitive_dependencies_helper TARGET TRANSITIVE_DEPS)
  if (NOT TARGET "${TARGET}")
    # Excluded from the project, or invalid name? Just ignore.
    return()
  endif()

  # Resolve aliases, canonicalize name formatting.
  get_target_property(_ALIASED_TARGET ${TARGET} ALIASED_TARGET)
  if(_ALIASED_TARGET)
    set(_TARGET_NAME ${_ALIASED_TARGET})
  else()
    string(REPLACE "::" "_" _TARGET_NAME ${TARGET})
  endif()

  set(_RESULT "${${TRANSITIVE_DEPS}}")
  if (${_TARGET_NAME} IN_LIST _RESULT)
    # Already visited, ignore.
    return()
  endif()

  # Append this target to the list. Dependencies of this target will be added
  # (if valid and not already visited) in recursive function calls.
  list(APPEND _RESULT ${_TARGET_NAME})

  # Check for non-target identifiers again after resolving the alias.
  if (NOT TARGET ${_TARGET_NAME})
    return()
  endif()

  # Get the list of direct dependencies for this target.
  get_target_property(_TARGET_TYPE ${_TARGET_NAME} TYPE)
  if(NOT ${_TARGET_TYPE} STREQUAL "INTERFACE_LIBRARY")
    get_target_property(_TARGET_DEPS ${_TARGET_NAME} LINK_LIBRARIES)
  else()
    get_target_property(_TARGET_DEPS ${_TARGET_NAME} INTERFACE_LINK_LIBRARIES)
  endif()

  if(_TARGET_DEPS)
    # Recurse on each dependency.
    foreach(_TARGET_DEP ${_TARGET_DEPS})
      _ockam_transitive_dependencies_helper(${_TARGET_DEP} _RESULT)
    endforeach(_TARGET_DEP)
  endif()

  # Propagate the augmented list up to the parent scope.
  set(${TRANSITIVE_DEPS} "${_RESULT}" PARENT_SCOPE)
endfunction()

# Sets target_link_libraries() on all registered binaries.
# This must be called after all libraries have been declared.
function(ockam_complete_binary_link_options)
  message(STATUS "------ ockam_complete_binary_link_options --------")
  get_property(_NAMES GLOBAL PROPERTY _OCKAM_CC_BINARY_NAMES)

  foreach(_NAME ${_NAMES})
    get_target_property(_DIRECT_DEPS ${_NAME} DIRECT_DEPS)

    # List all dependencies, including transitive dependencies, then split the
    # dependency list into one for whole archive (ALWAYSLINK) and one for
    # standard linking (which only links in symbols that are directly used).
    _ockam_transitive_dependencies("${_DIRECT_DEPS}" _TRANSITIVE_DEPS)
    set(_ALWAYS_LINK_DEPS "")
    set(_STANDARD_DEPS "")
    foreach(_DEP ${_TRANSITIVE_DEPS})
      # Check if _DEP is a library with the ALWAYSLINK property set.
      set(_DEP_IS_ALWAYSLINK OFF)
      if (TARGET ${_DEP})
        get_target_property(_DEP_TYPE ${_DEP} TYPE)
        if(${_DEP_TYPE} STREQUAL "INTERFACE_LIBRARY")
          # Can't be ALWAYSLINK since it's an INTERFACE library.
          # We also can't even query for the property, since it isn't whitelisted.
        else()
          get_target_property(_DEP_IS_ALWAYSLINK ${_DEP} ALWAYSLINK)
        endif()
      endif()

      # Append to the corresponding list of deps.
      if(_DEP_IS_ALWAYSLINK)
        list(APPEND _ALWAYS_LINK_DEPS ${_DEP})

        # For MSVC, also add a `-WHOLEARCHIVE:` version of the dep.
        # CMake treats -WHOLEARCHIVE[:lib] as a link flag and will not actually
        # try to link the library in, so we need the flag *and* the dependency.
        if(MSVC)
          get_target_property(_ALIASED_TARGET ${_DEP} ALIASED_TARGET)
          if (_ALIASED_TARGET)
            list(APPEND _ALWAYS_LINK_DEPS "-WHOLEARCHIVE:${_ALIASED_TARGET}")
          else()
            list(APPEND _ALWAYS_LINK_DEPS "-WHOLEARCHIVE:${_DEP}")
          endif()
        endif()
      else()
        list(APPEND _STANDARD_DEPS ${_DEP})
      endif()
    endforeach(_DEP)

    # Call into target_link_libraries with the lists of deps.
    if(MSVC)
      target_link_libraries(${_NAME}
        PUBLIC
          ${_ALWAYS_LINK_DEPS}
          ${_STANDARD_DEPS}
        PRIVATE
          ${_RULE_LINKOPTS}
      )
    else()
      if("${CMAKE_SYSTEM_NAME}" STREQUAL "Darwin")
        set(_ALWAYS_LINK_DEPS_W_FLAGS "-Wl,-all_load" ${_ALWAYS_LINK_DEPS} "-Wl")
      else()
        set(_ALWAYS_LINK_DEPS_W_FLAGS "-Wl,--whole-archive" ${_ALWAYS_LINK_DEPS} "-Wl,--no-whole-archive")
      endif()

      list(TRANSFORM _DIRECT_DEPS REPLACE "::" "_")
      list(TRANSFORM _DIRECT_DEPS PREPEND "_")
      target_link_directories(${_NAME} PUBLIC ${CMAKE_LIBRARY_OUTPUT_DIRECTORY})
      target_link_libraries(${_NAME}
        PUBLIC
          ${_ALWAYS_LINK_DEPS_W_FLAGS}
          ${_STANDARD_DEPS}
          ${_DIRECT_DEPS}
        PRIVATE
          ${_RULE_LINKOPTS}
      )
    endif()
  endforeach(_NAME)
endfunction()
