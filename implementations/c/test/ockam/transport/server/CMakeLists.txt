cmake_minimum_required(VERSION 3.14)
project(transport C)

set(CMAKE_C_STANDARD 99)

set(TRANSPORT_TEST_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(OCKAM_INC_DIR $ENV{OCKAM_C_BASE}/include/)
set(CMAKE_C_FLAGS "-std=c99 -g")

# set the project name
project(transport_server_test)

# Get the headers
include_directories( "./include/"
		${OCKAM_INC_DIR}
		$ENV{OCKAM_C_BASE}/source/ockam/transport/include/ )

# Add the source files
file(GLOB SOURCES "./*.c"
		$ENV{OCKAM_C_BASE}/source/ockam/log/*.c
		$ENV{OCKAM_C_BASE}/source/ockam/transport/server/*.c
		$ENV{OCKAM_C_BASE}/source/ockam/transport/*.c
		$ENV{OCKAM_C_BASE}/source/ockam/transport/connection/*.c
		$ENV{OCKAM_C_BASE}/source/ockam/transport/connection/socket/posix/*.c )


add_executable(transport_server_test ${SOURCES})
