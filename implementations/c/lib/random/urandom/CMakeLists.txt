# Ockam Random Urandom

project(urandom C)

message(STATUS "")
message(STATUS "------------- Ockam Random URandom ---------------")

ockam_cc_library(
   NAME
     urandom 
   SRCS
     "urandom.c"
   PUBLIC_HDRS
    "${CMAKE_CURRENT_SOURCE_DIR}/urandom.h"
   PUBLIC_HDRS_DIR
    "random/"
   PUBLIC)

