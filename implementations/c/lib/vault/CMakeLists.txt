# Add Ockam Vault implementations
add_subdirectory(default)
#add_subdirectory(atecc508a)
#add_subdirectory(atecc608a)

if(${OCKAM_BUILD_TESTS})
  add_subdirectory(tests)
endif()

# Ockam Vault
project(vault C)

message(STATUS "")
message(STATUS "------------------- Ockam Vault ------------------")

ockam_cc_library(
   NAME
     vault
   SRCS
     "vault.c"
   PUBLIC_HDRS
    "${CMAKE_CURRENT_SOURCE_DIR}/impl.h"
   PUBLIC_HDRS_DIR
    "vault/"
   PUBLIC)

