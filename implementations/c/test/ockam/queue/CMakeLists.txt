cmake_minimum_required(VERSION 3.10)

###########################
# Set directory locations #
###########################

set(QUEUE_TEST_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(OCKAM_INC_DIR $ENV{OCKAM_C_BASE}/include/ockam/)
set(CMAKE_C_FLAGS "-std=c99 -g")

# set the project name
project(test_queue)

# Get the headers
include_directories( ${OCKAM_INC_DIR}
			${CMAKE_CURRENT_SOURCE_DIR} )

# Add the source files
file(GLOB SOURCES "./*.c"
		$ENV{OCKAM_C_BASE}/source/ockam/queue/*.c
		$ENV{OCKAM_C_BASE}/source/ockam/log/*.c )

add_executable(test_queue ${SOURCES})
