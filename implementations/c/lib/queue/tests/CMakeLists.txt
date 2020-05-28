project(ockam_codec_tests C)
message(STATUS "----------------- Ockam Codec Tests ----------------")
ockam_cc_test(
	NAME
		full
	SRCS
		queue_test.c
	INCLUDES
		${CMOCKA_INCLUDE_DIRS}
	DEPS
		ockam::queue
		ockam::log
)
