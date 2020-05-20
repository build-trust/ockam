project(channel_test C)
message(STATUS "-----------------Channel Test----------------")

ockam_cc_test(
	NAME
		full
	SRCS
		channel_test.c
		initiator.c
		responder.c
		channel_test.h
	DEPS
		ockam::io
		ockam::transport
		ockam::log
		ockam::memory
		ockam::memory::stdlib
		ockam::random
		ockam::random::urandom
		ockam::key_agreement::xx
		ockam::channel
		ockam::vault
		ockam::vault::default
	TEST_OPTS
		-i -r
)
