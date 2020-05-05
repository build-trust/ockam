# Vault Test

project(ockam_test_vault C)

message(STATUS "")
message(STATUS "------------------- Vault Test -------------------")

ockam_cc_library(
  NAME
    suite
  SRCS
    "random.c"
    "sha256.c"
    "secret_ecdh.c"
    "hkdf.c"
    "aead_aes_gcm.c"
  INCLUDES
    ${CMOCKA_INCLUDE_DIRS}
  PUBLIC
)
