# Ockam Channel

project(channel C)

message(STATUS "------------------ key_agreement_xx ---------------")
ockam_cc_library(
	NAME
		channel
	SRCS
		channel_impl.c
	DEPS
		ockam::key_agreement::xx
)

if(${OCKAM_BUILD_TESTS})
	add_subdirectory(tests)
	message(STATUS "-----TEST posix_tcp_transport--------")
endif()
