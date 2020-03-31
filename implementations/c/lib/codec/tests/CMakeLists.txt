project(test_codec C)
message(STATUS "----------------- Ockam Codec Tests ----------------")
ockam_cc_test(
	NAME
		full
	SRCS
		"codec.c"
		"variable_length_encoded_u2le_test.c"
		"payload_aead_aes_gcm_test.c"
		"codec_tests.h"
	INCLUDES
		${CMOCKA_INCLUDE_DIRS}
	DEPS
		ockam::codec
		)
