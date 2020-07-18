
find_package(bearssl 0.6 QUIET)

if(NOT bearssl_FOUND)
  FetchContent_Declare(bearssl GIT_REPOSITORY https://www.bearssl.org/git/BearSSL GIT_TAG v0.6)

  FetchContent_GetProperties(bearssl)
  if(NOT bearssl_POPULATED)
    FetchContent_Populate(bearssl)
  endif()

  add_library(bearssl)

  target_include_directories(
    bearssl
    PUBLIC
      ${bearssl_SOURCE_DIR}/inc
    PRIVATE
      ${bearssl_SOURCE_DIR}/src
  )

  target_sources(
    bearssl
    PRIVATE
      ${bearssl_SOURCE_DIR}/src/aead/gcm.c
      ${bearssl_SOURCE_DIR}/src/codec/ccopy.c
      ${bearssl_SOURCE_DIR}/src/codec/dec32be.c
      ${bearssl_SOURCE_DIR}/src/codec/enc32be.c
      ${bearssl_SOURCE_DIR}/src/ec/ec_c25519_i31.c
      ${bearssl_SOURCE_DIR}/src/ec/ec_curve25519.c
      ${bearssl_SOURCE_DIR}/src/ec/ec_keygen.c
      ${bearssl_SOURCE_DIR}/src/ec/ec_p256_m31.c
      ${bearssl_SOURCE_DIR}/src/ec/ec_pubkey.c
      ${bearssl_SOURCE_DIR}/src/hash/sha2small.c
      ${bearssl_SOURCE_DIR}/src/hash/ghash_ctmul32.c
      ${bearssl_SOURCE_DIR}/src/int/i31_add.c
      ${bearssl_SOURCE_DIR}/src/int/i31_decmod.c
      ${bearssl_SOURCE_DIR}/src/int/i31_encode.c
      ${bearssl_SOURCE_DIR}/src/int/i31_montmul.c
      ${bearssl_SOURCE_DIR}/src/int/i31_sub.c
      ${bearssl_SOURCE_DIR}/src/kdf/hkdf.c
      ${bearssl_SOURCE_DIR}/src/mac/hmac.c
      ${bearssl_SOURCE_DIR}/src/rand/hmac_drbg.c
      ${bearssl_SOURCE_DIR}/src/symcipher/aes_ct.c
      ${bearssl_SOURCE_DIR}/src/symcipher/aes_ct_ctr.c
      ${bearssl_SOURCE_DIR}/src/symcipher/aes_ct_enc.c
  )

  # https://gitlab.kitware.com/cmake/cmake/-/issues/17735
  file(WRITE ${CMAKE_BINARY_DIR}/find/bearssl/bearsslConfig.cmake "")
  set(bearssl_DIR ${CMAKE_BINARY_DIR}/find/bearssl CACHE PATH "" FORCE)
endif()
