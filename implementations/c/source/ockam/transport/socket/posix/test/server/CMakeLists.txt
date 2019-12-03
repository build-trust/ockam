cmake_minimum_required(VERSION 3.10)

###########################
# Set directory locations #
###########################

set(SERVER_TEST_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(OCKAM_INC_DIR $ENV{OCKAM_C_BASE}/include/ockam/)
set(CMAKE_C_FLAGS "-std=c99 -g")

# set the project name
project(test_server)

# Get the headers
#include_directories( $ENV{OCKAM_C_BASE}/include/ockam/
#        $ENV{OCKAM_C_BASE}/source/ockam/transport/include/)
include_directories( ${OCKAM_INC_DIR}
        $ENV{OCKAM_C_BASE}/source/ockam/transport/include/)

# Add the source files
file(GLOB SOURCES "./*.c" "../*.c"
        $ENV{OCKAM_C_BASE}/source/ockam/transport/socket/posix/source/*.c)

add_executable(test_server ${SOURCES})
