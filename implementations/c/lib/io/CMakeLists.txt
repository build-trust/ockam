project(io C)

message(STATUS "------------------ ockam_io---------------")
ockam_cc_library(
	NAME
		io
	SRCS
		"io_impl.h"
		"io_impl.c"
	PUBLIC
)
