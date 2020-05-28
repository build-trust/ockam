project(ockam_codec_tests C)
message(STATUS ----------------- Ockam Codec Tests ----------------)
ockam_cc_test(
	NAME
		full
	SRCS
		codec.c
		variable_length_encoded_u2le_test.c
		payload_aead_aes_gcm_test.c
		public_key_test.c
		payload_test.c
		endpoint_local_test.c
		endpoint_channel_test.c
		endpoint_test.c
		codec_tests.h
		route_test.c
	INCLUDES
		${CMOCKA_INCLUDE_DIRS}
	DEPS
		ockam::codec
)
