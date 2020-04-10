# Ockam Transport Test
project(test_tcp_transport C)
message(STATUS "-------------- Ockam Transport Test --------------")

ockam_cc_test(
  NAME
    full
  SRCS
    "test_tcp_server.c"
    "test_tcp_client.c"
    "test_tcp_tools.c"
  DEPS
    ockam::transport::posix_socket::tcp
    ockam::log
  TEST_OPTS
    -f ${OCKAM_TRANSPORT_TEST_FIXTURE_DIR}
)
