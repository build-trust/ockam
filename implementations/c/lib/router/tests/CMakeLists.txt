project(test_router C)

ockam_cc_test(
	NAME
		full
	SRCS
		router_test.c
	DEPS
		ockam::io
		ockam::memory::stdlib
		ockam::memory
		ockam::transport
		ockam::log
		ockam::channel
	TEST_OPTS
		-i -r
)
