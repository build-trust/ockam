# Ockam Memory

project(memory C)

message(STATUS "")
message(STATUS "------------------ Ockam Memory ------------------")

ockam_cc_library(
   NAME
     stdlib
   SRCS
     "stdlib.c"
   PUBLIC_HDRS
    "${CMAKE_CURRENT_SOURCE_DIR}/stdlib.h"
   PUBLIC_HDRS_DIR
    "memory/"
   DEPS
     ockam::memory
   PUBLIC)

if(${OCKAM_BUILD_TESTS})
    add_subdirectory(tests)
    message(STATUS "-----TEST ockam_codec--------")
endif()