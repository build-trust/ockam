project(test_interfaces C)

ockam_cc_test(
	NAME
		full
	SRCS
		client.c
		server.c
		tools.c
		tests.h
	DEPS
		ockam::io
#		ockam::memory::stdlib
#		ockam::memory
		ockam::transport
		ockam::log
		ockam::memory
		ockam::memory::stdlib
	TEST_OPTS
		-f ${OCKAM_TRANSPORT_TEST_FIXTURE_DIR}
)
