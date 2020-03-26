# Ockam Memory

project(memory C)

message(STATUS "")
message(STATUS "------------------ Ockam Memory ------------------")

ockam_cc_library(
   NAME
     memory
   SRCS
     "stdlib.c"
   PUBLIC )
