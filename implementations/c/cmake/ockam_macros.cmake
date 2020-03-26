include(CMakeParseArguments)

#-------------------------------------------------------------------------------
# Packages and Paths
#-------------------------------------------------------------------------------

# Sets ${PACKAGE_NS} to the Ockam-root relative package name in C++ namespace
# format (::).
#
# Example when called from implementations/c/foo/CMakeLists.txt:
#   ockam::foo
function(ockam_package_ns PACKAGE_NS)
  if(${CMAKE_CURRENT_LIST_DIR} MATCHES "\/lib\/")
    string(REPLACE "${OCKAM_LIB_DIR}" "ockam" _PACKAGE ${CMAKE_CURRENT_LIST_DIR})
  elseif(${CMAKE_CURRENT_LIST_DIR} MATCHES "\/tests\/")
    string(REPLACE "${OCKAM_C_ROOT_DIR}" "ockam" _PACKAGE ${CMAKE_CURRENT_LIST_DIR})
  endif()

  string(REPLACE "/" "::" _PACKAGE_NS ${_PACKAGE})
  set(${PACKAGE_NS} "${_PACKAGE_NS}" PARENT_SCOPE)
endfunction()

# Sets ${PACKAGE_NAME} to the Ockam-root relative package name.
#
# Example when called from implementations/c/foo/CMakeLists.txt:
#   ockam_foo
function(ockam_package_name PACKAGE_NAME)
  ockam_package_ns(_PACKAGE_NS)
  string(REPLACE "::" "_" _PACKAGE_NAME "${_PACKAGE_NS}")
  set(${PACKAGE_NAME} ${_PACKAGE_NAME} PARENT_SCOPE)
endfunction()

# Sets ${PACKAGE_PATH} to the Ockam-root relative package path.
#
# Example when called from implementations/c/foo/bar/CMakeLists.txt:
#   foo/bar
function(ockam_package_path PACKAGE_PATH)
  string(REPLACE ${OCKAM_C_ROOT_DIR} "" _PACKAGE ${CMAKE_CURRENT_LIST_DIR})
  string(SUBSTRING "${_PACKAGE}" 1 -1 _PACKAGE_PATH)
  set(${PACKAGE_PATH} ${_PACKAGE_PATH} PARENT_SCOPE)
endfunction()

# Sets ${PACKAGE_DIR} to the directory name of the current package.
#
# Example when called from ockam/foo/CMakeLists.txt:
#   foo
function(ockam_package_dir PACKAGE_DIR)
  ockam_package_ns(_PACKAGE_NS)
  string(FIND "${_PACKAGE_NS}" "::" _END_OFFSET REVERSE)
  math(EXPR _END_OFFSET "${_END_OFFSET} + 2")
  string(SUBSTRING "${_PACKAGE_NS}" ${_END_OFFSET} -1 _PACKAGE_DIR)
  set(${PACKAGE_DIR} ${_PACKAGE_DIR} PARENT_SCOPE)
endfunction()

#-------------------------------------------------------------------------------
# select()-like Evaluation
#-------------------------------------------------------------------------------

# Appends ${OPTS} with a list of values based on the current compiler.
#
# Example:
#   ockam_select_compiler_opts(COPTS
#     CLANG
#       "-Wno-foo"
#       "-Wno-bar"
#     CLANG_CL
#       "/W3"
#     GCC
#       "-Wsome-old-flag"
#     MSVC
#       "/W3"
#   )
#
# Note that variables are allowed, making it possible to share options between
# different compiler targets.
function(ockam_select_compiler_opts OPTS)
  cmake_parse_arguments(
    PARSE_ARGV 1
    _OCKAM_SELECTS
    ""
    ""
    "ALL;CLANG;CLANG_CL;MSVC;GCC;CLANG_OR_GCC;MSVC_OR_CLANG_CL"
  )
  set(_OPTS)
  list(APPEND _OPTS "${_OCKAM_SELECTS_ALL}")
  if("${CMAKE_CXX_COMPILER_ID}" STREQUAL "GNU")
    list(APPEND _OPTS "${_OCKAM_SELECTS_GCC}")
    list(APPEND _OPTS "${_OCKAM_SELECTS_CLANG_OR_GCC}")
  elseif("${CMAKE_CXX_COMPILER_ID}" MATCHES "Clang")
    if(MSVC)
      list(APPEND _OPTS ${_OCKAM_SELECTS_CLANG_CL})
      list(APPEND _OPTS ${_OCKAM_SELECTS_MSVC_OR_CLANG_CL})
    else()
      list(APPEND _OPTS ${_OCKAM_SELECTS_CLANG})
      list(APPEND _OPTS ${_OCKAM_SELECTS_CLANG_OR_GCC})
    endif()
  elseif("${CMAKE_CXX_COMPILER_ID}" STREQUAL "MSVC")
    list(APPEND _OPTS ${_OCKAM_SELECTS_MSVC})
    list(APPEND _OPTS ${_OCKAM_SELECTS_MSVC_OR_CLANG_CL})
  else()
    message(ERROR "Unknown compiler: ${CMAKE_CXX_COMPILER}")
    list(APPEND _OPTS "")
  endif()
  set(${OPTS} ${_OPTS} PARENT_SCOPE)
endfunction()
