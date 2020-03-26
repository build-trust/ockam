# Ockam Log

project(log C)

message(STATUS "")
message(STATUS "------------------- Ockam Log --------------------")
ockam_cc_library(
   NAME
     log
   SRCS
     "printf.c"
     "syslog.c"
   PUBLIC )
