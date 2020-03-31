# Ockam Transport

project(codec C)

message(STATUS "------------------ ockam_codec---------------")
ockam_cc_library(
	NAME
		codec
	SRCS
		"variable_length_encoded_u2le.c"
		"payload_aead_aes_gcm.c"
	HDRS
  PUBLIC )

if(${OCKAM_BUILD_TESTS})
	add_subdirectory(tests)
endif()
