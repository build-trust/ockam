# Ockam Vault BearSSL

project(test_atecc608a C)

message(STATUS "")
message(STATUS "----------------- Test ATECC608A -----------------")

ockam_cc_test(
   NAME
     full 
   SRCS
     "test_atecc608a.c"
   INCLUDES
     "${cryptoauthlib-git_SOURCE_DIR}/lib"
     "${OCKAM_C_ROOT_DIR}/lib/vault/tests"
     "${OCKAM_C_ROOT_DIR}/lib/vault/atecc608a"
   DEPS 
     ockam::vault::tests::suite
     ockam::vault::atecc608a
     ockam::memory
   PUBLIC)
