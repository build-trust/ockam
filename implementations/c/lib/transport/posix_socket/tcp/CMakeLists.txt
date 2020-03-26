# Ockam Transport

project(tcp C)

message(STATUS "------------------ ockam_transport ---------------")
ockam_cc_library(
   NAME
     tcp
   SRCS
     "../posix_socket.c"
     "posix_tcp.c"
		"posix_tcp.h"
  HDRS
    "../posix_socket.h"
    "server.h"
   PUBLIC )
