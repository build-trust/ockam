if (NOT ${OCKAM_BUILD_TESTS})
  return()
endif()

include(FetchContent)

FetchContent_Declare(cmocka
  URL https://cmocka.org/files/1.1/cmocka-1.1.5.tar.xz
  URL_HASH MD5=91f95cd5db88b9b120d191b18d367193
  QUIET
  SOURCE_DIR "${OCKAM_TESTS_DIR}/cmocka"
)

FetchContent_GetProperties(cmocka)

if(NOT cmocka_POPULATED)
  FetchContent_Populate(cmocka)
endif()

set(WITH_STATIC_LIB ON CACHE BOOL "Build with a static library")
set(WITH_EXAMPLES OFF CACHE BOOL "Build examples")

add_subdirectory(${cmocka_SOURCE_DIR} ${cmocka_BINARY_DIR} EXCLUDE_FROM_ALL)
set(CMOCKA_INCLUDE_DIRS "${cmocka_SOURCE_DIR}/include")
