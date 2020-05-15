project(io C)

message(STATUS "------------------ ockam_io---------------")
ockam_cc_library(
	NAME
		io
	SRCS
		"../../include/ockam/io/io_impl.h"
		"io_impl.c"
	PUBLIC
)
