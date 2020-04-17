#-------------------------------------------------------------------------------
# C/C++ used within Ockam
#-------------------------------------------------------------------------------

set(OCKAM_C_STANDARD 99)
set(OCKAM_CXX_STANDARD 17)

set(OCKAM_LIB_DIR "${OCKAM_C_ROOT_DIR}/lib")
set(OCKAM_TESTS_DIR "${OCKAM_C_ROOT_DIR}/tests")
set(CMAKE_EXPORT_COMPILE_COMMANDS OFF CACHE BOOL "")

list(APPEND OCKAM_COMMON_INCLUDE_DIRS
  ${OCKAM_C_ROOT_DIR}/include
  ${OCKAM_INCLUDES_OUTPUT_DIRECTORY}
  ${CMAKE_CURRENT_BINARY_DIR}
)

ockam_select_compiler_opts(OCKAM_DEFAULT_COPTS
  CLANG
    "-Wno-strict-prototypes"
    "-Wno-shadow-uncaptured-local"
    "-Wno-gnu-zero-variadic-macro-arguments"
    "-Wno-shadow-field-in-constructor"
    "-Wno-unreachable-code-return"
    "-Wno-unused-private-field"
    "-Wno-missing-variable-declarations"
    "-Wno-gnu-label-as-value"
    "-Wno-unused-local-typedef"
    "-Wno-gnu-zero-variadic-macro-arguments"
  CLANG_OR_GCC
    "-Wno-unused-parameter"
    "-Wno-undef"
    "-Werror"
  MSVC_OR_CLANG_CL
    "/DWIN32_LEAN_AND_MEAN"
    "/EHsc"
)

set(OCKAM_DEFAULT_LINKOPTS "")
set(OCKAM_TEST_COPTS "")

#-------------------------------------------------------------------------------
# Compiler: Clang/LLVM
#-------------------------------------------------------------------------------

# TODO: Clang/LLVM options.

#-------------------------------------------------------------------------------
# Compiler: GCC
#-------------------------------------------------------------------------------

# TODO: GCC options.

#-------------------------------------------------------------------------------
# Compiler: MSVC
#-------------------------------------------------------------------------------

# TODO: MSVC options.

#-------------------------------------------------------------------------------
# Third party: benchmark
#-------------------------------------------------------------------------------

set(BENCHMARK_ENABLE_TESTING OFF CACHE BOOL "" FORCE)
set(BENCHMARK_ENABLE_INSTALL OFF CACHE BOOL "" FORCE)

#-------------------------------------------------------------------------------
# Third party: cmocka
#-------------------------------------------------------------------------------

#include(ockam_cmocka)
