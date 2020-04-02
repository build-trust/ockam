# Ockam Vault ATECC608A

project(atecc608a C)
include(FetchContent)

message(STATUS "")
message(STATUS "---------------- Vault Atecc608a -----------------")

set(ATCA_HAL_I2C ON CACHE BOOL "")
set(ATCA_BUILD_SHARED_LIBS OFF CACHE BOOL "")

FetchContent_Declare(
  cryptoauthlib-tar
  URL https://github.com/MicrochipTech/cryptoauthlib/releases/download/v3.1.0/cryptauthlib-20200205.tar.gz
  URL_HASH MD5=e6e1fec30194c778ff5798b85e5db66a
  QUIET
)

FetchContent_MakeAvailable(cryptoauthlib-tar)

ockam_cc_library(
  NAME
    atecc608a
  SRCS
    "atecc608a.c"
  INCLUDES
    "${cryptoauthlib-tar_SOURCE_DIR}/lib"
    "${cryptoauthlib-tar_SOURCE_DIR}/lib/hal"
    "${cryptoauthlib-tar_BINARY_DIR}/lib"
  DEPS
    cryptoauth
    ockam::memory
  PUBLIC
)

if(${OCKAM_BUILD_TESTS})
  add_subdirectory(tests)
endif()

