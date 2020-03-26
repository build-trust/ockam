# Vault Test

project(ockam_test_vault C)

message(STATUS "")
message(STATUS "------------------- Vault Test -------------------")

ockam_cc_library(
  NAME
    suite
  SRCS
    "aes_gcm.c"
    "hkdf.c"
    "key_ecdh.c"
    "random.c"
    "sha256.c"
  INCLUDES
    ${CMOCKA_INCLUDE_DIRS}
  PUBLIC
)
