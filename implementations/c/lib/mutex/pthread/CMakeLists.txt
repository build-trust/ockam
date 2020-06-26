# Ockam Mutex Pthread

project(mutex_pthread C)

message(STATUS "")
message(STATUS "--------------- Ockam Mutex Pthread --------------")

ockam_cc_library(
   NAME
     pthread
   SRCS
     "pthread.c"
   PUBLIC_HDRS
    "${CMAKE_CURRENT_SOURCE_DIR}/pthread.h"
   PUBLIC_HDRS_DIR
    "mutex/"
   DEPS
     ockam::mutex
   PUBLIC)

