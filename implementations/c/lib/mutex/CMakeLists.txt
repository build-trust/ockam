# Add Ockam Memory implementations
add_subdirectory(pthread)

# Ockam Mutex
project(mutex C)

message(STATUS "")
message(STATUS "------------------ Ockam Mutex -------------------")

ockam_cc_library(
   NAME
     mutex
   SRCS
     "mutex.c"
   PUBLIC_HDRS
    "${CMAKE_CURRENT_SOURCE_DIR}/impl.h"
   PUBLIC_HDRS_DIR
    "mutex/"
   PUBLIC)

