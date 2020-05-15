project(test_key_agreement_xx C)
message(STATUS "-----------------Key Agreement XX ----------------")
#ockam_cc_binary(NAME
ockam_cc_test(NAME
		full
	SRCS
		"xx_test_initiator.c"
		"xx_test_responder.c"
		"xx_test.c"
		"xx_test.h"
	DEPS
		ockam::io
		ockam::transport
		ockam::log
		ockam::memory
		ockam::memory::stdlib
		ockam::random
		ockam::random::urandom
		ockam::key_agreement::xx
		ockam::vault
		ockam::vault::default
	)
