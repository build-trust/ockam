project(router C)

message(STATUS "------------------ ockam_router---------------")
ockam_cc_library(
	NAME
		router
	SRCS
		router.c
		../../include/ockam/router.h
	PUBLIC
)

if(${OCKAM_BUILD_TESTS})
	add_subdirectory(tests)
	message(STATUS "-----TEST ockam_router--------")
endif()
