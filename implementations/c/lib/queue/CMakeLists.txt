# Ockam queue

project(queue C)

message(STATUS "")
message(STATUS "------------------- Ockam Log --------------------")
ockam_cc_library(
	NAME
		queue
	SRCS
		queue.c
	DEPS
		ockam::memory
		ockam::memory::stdlib
	PUBLIC )

if(${OCKAM_BUILD_TESTS})
	add_subdirectory(tests)
	message(STATUS "-----TEST ockam_codec--------")
endif()
