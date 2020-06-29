project(ockam_linked_list_tests C)
message(STATUS "----------------- Ockam Codec Tests ----------------")
ockam_cc_test(
	NAME
		full
	SRCS
		linked_list_test.c
	INCLUDES
	DEPS
		ockam::memory
		ockam::memory::stdlib
		ockam::queue
		ockam::log
		ockam::linked_list
)
