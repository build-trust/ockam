# Ockam queue

project(queue C)

message(STATUS "")
message(STATUS "------------------- Ockam Log --------------------")
ockam_cc_library(
	NAME
		linked_list
	SRCS
		linked_list.c
	DEPS
		ockam::memory
		ockam::memory::stdlib
	PUBLIC )

if(${OCKAM_BUILD_TESTS})
	add_subdirectory(tests)
	message(STATUS "-----TEST linked_lisd--------")
endif()