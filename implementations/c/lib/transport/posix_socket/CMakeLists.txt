# Ockam Transport

project(transport C)

message(STATUS "------------------ ockam_transport ---------------")
ockam_cc_library(
	NAME
		transport
	SRCS
		socket.c
		socket.h
		socket_tcp.c
		socket_tcp.h
#	HDRS
#		transport/socket.h
	PUBLIC )

if(${OCKAM_BUILD_TESTS})
	add_subdirectory(tests)
	message(STATUS "-----TEST posix_tcp_transport--------")
endif()
