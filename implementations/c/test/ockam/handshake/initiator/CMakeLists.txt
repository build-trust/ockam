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

project(handshake_initiator_test)


###########################
# Set directory locations #
###########################

set(TEST_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(TEST_CFG_DIR ${CMAKE_CURRENT_SOURCE_DIR}/config)

set(TEST_INC_DIR ${OCKAM_C_BASE}/test/ockam/handshake/include)

set(OCKAM_SRC_DIR ${OCKAM_C_BASE}/source/ockam)
set(OCKAM_INC_DIR ${OCKAM_C_BASE}/include)

set(VAULT_SRC_DIR ${OCKAM_SRC_DIR}/vault)
set(KAL_SRC_DIR ${OCKAM_SRC_DIR}/kal)
set(LOG_SRC_DIR ${OCKAM_SRC_DIR}/log)
set(MEM_SRC_DIR ${OCKAM_SRC_DIR}/memory)
set(TRANSPORT_SRC_DIR ${OCKAM_SRC_DIR}/transport)
set(HANDSHAKE_SRC_DIR ${OCKAM_SRC_DIR}/handshake)

set(THIRD_PARTY_DIR ${OCKAM_C_BASE}/third-party)

set(BINARY_DIR _build/$ENV{PLATFORM})

#################
# Build Options #
#################

# Vault Build Options
set(VAULT_HOST_MBEDCRYPTO TRUE)

# KAL Build Option
set(KAL_LINUX TRUE)

# Log Build Option
set(LOG_PRINTF TRUE)

# Mem Build Option
set(MEM_STDLIB TRUE)

# Compiler Build Options
set(CMAKE_VERBOSE_MAKEFILE TRUE)


###########################
# Set include directories #
###########################

set(TEST_INC ${TEST_INC} ${OCKAM_INC_DIR})
set(TEST_INC ${TEST_INC} ${TEST_INC_DIR})
set(TEST_INC ${TEST_INC} ${HANDSHAKE_INC_DIR})

include_directories(${TEST_INC})

####################
# Set config files #
####################

add_definitions(-DOCKAM_VAULT_CONFIG_FILE="${TEST_SRC_DIR}/config/vault_config.h")

####################
# Set source files #
####################

#set(TEST_SRC ${TEST_SRC_DIR}/handshake_initiator_test.c)
set(TEST_SRC ${CMAKE_CURRENT_SOURCE_DIR}/handshake_initiator_test.c)

###########################
# Set the desired modules #
###########################

add_subdirectory(${VAULT_SRC_DIR} vault)
add_subdirectory(${KAL_SRC_DIR} kal)
add_subdirectory(${LOG_SRC_DIR} log)
add_subdirectory(${MEM_SRC_DIR} mem)
add_subdirectory(${TRANSPORT_SRC_DIR} transport)
add_subdirectory(${HANDSHAKE_SRC_DIR} handshake)

#########################################
# Configure link libraries & executable #
#########################################

link_directories(${CMAKE_CURRENT_SOURCE_DIR}/${BINARY_DIR})
add_executable(handshake_initiator_test ${TEST_SRC})

target_link_libraries(handshake_initiator_test ockam_vault)
target_link_libraries(handshake_initiator_test ockam_kal)
target_link_libraries(handshake_initiator_test ockam_log)
target_link_libraries(handshake_initiator_test mbedcrypto)
target_link_libraries(handshake_initiator_test ockam_transport)
target_link_libraries(handshake_initiator_test ockam_handshake)

install(TARGETS handshake_initiator_test DESTINATION bin)
