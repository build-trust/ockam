# Ockam Vault Default

project(default C)
include(FetchContent)

message(STATUS "")
message(STATUS "----------------- Vault Default  -----------------")

set(BEARSSL_DIR ${CMAKE_CURRENT_SOURCE_DIR}/bearssl-0.6)

FetchContent_Declare(bearssl-tar
  URL https://bearssl.org/bearssl-0.6.tar.gz
  URL_HASH MD5=1513f9828c5b174ea409ca581cb45c98
  QUIET
  SOURCE_DIR "${BEARSSL_DIR}"
)

FetchContent_GetProperties(bearssl-tar)

if(NOT bearssl-tar_POPULATED)
  message(STATUS "FETCHING PACKAGE               : BearSSL") 
  FetchContent_Populate(bearssl-tar)
endif()

ockam_cc_library(
   NAME
     default 
   SRCS
     "default.c"
     "${BEARSSL_DIR}/src/aead/gcm.c"
     "${BEARSSL_DIR}/src/codec/ccopy.c"
     "${BEARSSL_DIR}/src/codec/dec32be.c"
     "${BEARSSL_DIR}/src/codec/enc32be.c"
     "${BEARSSL_DIR}/src/ec/ec_c25519_i31.c"
     "${BEARSSL_DIR}/src/ec/ec_curve25519.c"
     "${BEARSSL_DIR}/src/ec/ec_keygen.c"
     "${BEARSSL_DIR}/src/ec/ec_p256_m31.c"
     "${BEARSSL_DIR}/src/ec/ec_pubkey.c"
     "${BEARSSL_DIR}/src/hash/sha2small.c"
     "${BEARSSL_DIR}/src/hash/ghash_ctmul32.c"
     "${BEARSSL_DIR}/src/int/i31_add.c"
     "${BEARSSL_DIR}/src/int/i31_decmod.c"
     "${BEARSSL_DIR}/src/int/i31_encode.c"
     "${BEARSSL_DIR}/src/int/i31_montmul.c"
     "${BEARSSL_DIR}/src/int/i31_sub.c"
     "${BEARSSL_DIR}/src/kdf/hkdf.c"
     "${BEARSSL_DIR}/src/mac/hmac.c"
     "${BEARSSL_DIR}/src/rand/hmac_drbg.c"
     "${BEARSSL_DIR}/src/symcipher/aes_ct.c"
     "${BEARSSL_DIR}/src/symcipher/aes_ct_ctr.c"
     "${BEARSSL_DIR}/src/symcipher/aes_ct_enc.c"
   INCLUDES
     "${BEARSSL_DIR}/inc"
     "${BEARSSL_DIR}/src"
   DEPS
     ockam::memory
   PUBLIC )


if(${OCKAM_BUILD_TESTS})
  add_subdirectory(tests)
endif()

