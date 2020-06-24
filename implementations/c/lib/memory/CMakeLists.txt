# Add Ockam Memory implementations
add_subdirectory(stdlib)

# Ockam Memory
project(memory C)

message(STATUS "")
message(STATUS "------------------ Ockam Memory ------------------")

ockam_cc_library(
   NAME
     memory
   SRCS
     "memory.c"
   PUBLIC_HDRS
    "${CMAKE_CURRENT_SOURCE_DIR}/impl.h"
   PUBLIC_HDRS_DIR
    "memory/"
   PUBLIC)

if(${OCKAM_BUILD_TESTS})
    add_subdirectory(tests)
endif()