# Ockam Transport

project(codec C)

message(STATUS "------------------ ockam_codec---------------")
ockam_cc_library(
	NAME
		codec
	SRCS
		variable_length_encoded_u2le.c
		payload_aead_aes_gcm.c
		public_key.c
		payload.c
		endpoint.c
		key_agreement.c
		ockam_wire.c
		route.c
)

if(${OCKAM_BUILD_TESTS})
	add_subdirectory(tests)
	message(STATUS "-----TEST ockam_codec--------")
endif()


