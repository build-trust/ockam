project(test_interfaces C)

set(
	COMMON_SRCS
		runner.c
		client.c
		server.c
		tools.c
		runner.h
		client.h
		server.h
		tools.h
)

ockam_cc_test(
	NAME
		tcp
	SRCS
		${COMMON_SRCS}
		udp_test.c
	DEPS
		ockam::io
		ockam::transport
		ockam::log
		ockam::memory
		ockam::memory::stdlib
	TEST_OPTS
		-f${OCKAM_TRANSPORT_TEST_FIXTURE_DIR}
)

ockam_cc_test(
	NAME
		udp
	SRCS
		${COMMON_SRCS}
		tcp_test.c
	DEPS
		ockam::io
		ockam::transport
		ockam::log
		ockam::memory
		ockam::memory::stdlib
	TEST_OPTS
		-f${OCKAM_TRANSPORT_TEST_FIXTURE_DIR}
)
