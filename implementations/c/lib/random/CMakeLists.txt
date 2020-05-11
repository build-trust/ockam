# Add Ockam Random implementations
add_subdirectory(urandom)

# Ockam Random
project(random C)

message(STATUS "")
message(STATUS "------------------ Ockam Random ------------------")

ockam_cc_library(
   NAME
     random
   SRCS
     "random.c"
   PUBLIC_HDRS
    "${CMAKE_CURRENT_SOURCE_DIR}/impl.h"
   PUBLIC_HDRS_DIR
    "random/"
   PUBLIC)

