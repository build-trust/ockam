project(test_interfaces C)

ockam_cc_test(
	NAME
		full
	SRCS
		transport_test.c
		client.c
		server.c
		tools.c
		tests.h
	DEPS
		ockam::io
		ockam::transport
		ockam::log
		ockam::memory
		ockam::memory::stdlib
	TEST_OPTS
		-c -s -f${OCKAM_TRANSPORT_TEST_FIXTURE_DIR}
)
