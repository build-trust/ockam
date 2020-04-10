# Ockam Vault ATECC508A

# Only build for the Pi
if(NOT "${OCKAM_TARGET_VENDOR}" MATCHES "rpi3")
  return()
endif()

project(atecc508a C)
include(FetchContent)

message(STATUS "")
message(STATUS "---------------- Vault Atecc508a -----------------")

set(ATCA_HAL_I2C ON CACHE BOOL "")
set(ATCA_BUILD_SHARED_LIBS OFF CACHE BOOL "")

FetchContent_Declare(
  cryptoauthlib-tar
  URL https://github.com/MicrochipTech/cryptoauthlib/releases/download/v3.1.0/cryptauthlib-20200205.tar.gz
  URL_HASH MD5=e6e1fec30194c778ff5798b85e5db66a
  QUIET
)

FetchContent_MakeAvailable(cryptoauthlib-tar)

#FetchContent_GetProperties(cryptoauthlibgit)
#
#if(NOT cryptoauthlibgit_POPULATED)
#  message(STATUS "||||| DOWNLOADING |||||")
#  FetchContent_Populate(cryptoauthlibgit)
#endif()

#add_subdirectory(${cryptoauthlibgit_SOURCE_DIR} ${cryptoauthlibgit_BINARY_DIR})

ockam_cc_library(
  NAME
    atecc508a
  SRCS
    "atecc508a.c"
  INCLUDES
    "${cryptoauthlib-tar_SOURCE_DIR}/lib"
    "${cryptoauthlib-tar_SOURCE_DIR}/lib/hal"
    "${cryptoauthlib-tar_BINARY_DIR}/lib"
     "${OCKAM_C_ROOT_DIR}/lib/vault/default"
  PUBLIC_HDRS
    "${CMAKE_CURRENT_SOURCE_DIR}/atecc508a.h"
  PUBLIC_HDRS_DIR
    "vault/"
  DEPS
    cryptoauth
    ockam::memory
    ockam::vault::default
  PUBLIC
)

if(${OCKAM_BUILD_TESTS})
  add_subdirectory(tests)
endif()

