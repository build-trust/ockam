cmake_minimum_required(VERSION 3.14)

###########################
# Path & Compiler Options #
###########################

# Always load the path.cmake file FIRST
include($ENV{OCKAM_C_BASE}/tools/cmake/path.cmake)

# This must be included BEFORE the project declaration

if($ENV{PLATFORM} MATCHES "PI")
	message("Building for PI")
	include(${OCKAM_C_BASE}/tools/cmake/toolchains/raspberry-pi.cmake)
else()
	set(CMAKE_C_STANDARD 99)
	set(CMAKE_C_FLAGS "-std=c99 -g")
endif()

###########
# Project #
###########

project(transport_server_test)

###########################
# Set directory locations #
###########################

set(TEST_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(OCKAM_SRC_DIR ${OCKAM_C_BASE}/source/ockam)
set(OCKAM_INC_DIR ${OCKAM_C_BASE}/include)

set(TRANSPORT_SRC_DIR ${OCKAM_SRC_DIR}/transport)
set(BINARY_DIR _build/$ENV{PLATFORM})

#################
# Build Options #
#################

# Compiler Build Options
set(CMAKE_VERBOSE_MAKEFILE TRUE)

###########################
# Set include directories #
###########################

set(TEST_INC ${TEST_INC} ${OCKAM_INC_DIR})
set(TEST_INC ${TEST_INC} ${TEST_INC_DIR})

include_directories(${TEST_INC})

# Get the headers
#include_directories( "./include/"
#  ${OCKAM_INC_DIR}
#  $ENV{OCKAM_C_BASE}/source/ockam/transport/include/ )

####################
# Set source files #
####################

set(TEST_SRC ${CMAKE_CURRENT_SOURCE_DIR}/transport_server_test.c
  ${OCKAM_SRC_DIR}/log/syslog.c)

set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/${BINARY_DIR})

###########################
# Set the desired modules #
###########################

add_subdirectory(${TRANSPORT_SRC_DIR} ${BINARY_DIR})

#########################################
# Configure link libraries & executable #
#########################################

link_directories(${CMAKE_CURRENT_SOURCE_DIR}/${BINARY_DIR})
add_executable(transport_server_test ${TEST_SRC})
target_link_libraries(transport_server_test ockam_transport)

install(TARGETS transport_server_test DESTINATION bin)
