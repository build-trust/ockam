cmake_minimum_required(VERSION 3.15)

##########################
# Project handshake_responder_test #
##########################

###########################
# Path & Compiler Options #
###########################

# Always load the path.cmake file FIRST
include($ENV{OCKAM_C_BASE}/tools/cmake/path.cmake)

# This must be included BEFORE the project declaration
if($ENV{PLATFORM} MATCHES "PI")
	 message("********Building for PI**********")
   include(${OCKAM_C_BASE}/tools/cmake/toolchains/raspberry-pi.cmake)
else()
	set(CMAKE_C_STANDARD 99)
	set(CMAKE_C_FLAGS "-std=c99 -g")
endif()

project(handshake_responder_test C )

###########################
# Set directory locations #
###########################

set(TEST_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(TEST_CFG_DIR ${TEST_SRC_DIR}/config)

set(COMMON_OCKAM_SRC_DIR $ENV{OCKAM_C_BASE}/source/ockam)
set(XX_HANDSHAKE_RESPONDER_TEST_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})

set(OCKAM_SRC_DIR ${OCKAM_C_BASE}/source/ockam)
set(OCKAM_INC_DIR $ENV{OCKAM_C_BASE}/include/)
set(BINARY_DIR _build/$ENV{PLATFORM})

set(VAULT_SRC_DIR ${OCKAM_SRC_DIR}/vault)
set(KAL_SRC_DIR ${OCKAM_SRC_DIR}/kal)
set(LOG_SRC_DIR ${OCKAM_SRC_DIR}/log)
set(MEM_SRC_DIR ${OCKAM_SRC_DIR}/memory)
set(TRANSPORT_POSIX_SRC_DIR ${COMMON_OCKAM_SRC_DIR}/transport)
set(XX_HANDSHAKE_SRC_DIR ${COMMON_OCKAM_SRC_DIR}/handshake)

set(THIRD_PARTY_DIR ${OCKAM_C_BASE}/third-party)

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
#set(CMAKE_VERBOSE_MAKEFILE TRUE)

###########################
# Set include directories #
###########################

set(TEST_INC ${TEST_INC} ${OCKAM_INC_DIR})
set(TEST_INC ${TEST_INC} ${TEST_COMMON_INC_DIR})
<<<<<<< HEAD
set(TEST_INC ${TEST_INC} ${XX_HANDSHAKE_SRC_DIR}/include)
=======
>>>>>>> 2a3b2b50c47508716005292b5f54c7c1048be7c3

include_directories(${TEST_INC} "./" "./../include"
		${OCKAM_INC_DIR}
		${OCKAM_C_BASE}/source/ockam/transport/include/ )


####################
# Set config files #
####################

add_definitions(-DOCKAM_VAULT_CONFIG_FILE="${TEST_CFG_DIR}/vault_config.h")

##############################
# Set transport source files #
##############################

file(GLOB SOURCES "./*.c"
		$ENV{OCKAM_C_BASE}/source/ockam/transport/*.c
		$ENV{OCKAM_C_BASE}/source/ockam/transport/connection/*.c
		$ENV{OCKAM_C_BASE}/source/ockam/transport/connection/socket/posix/*.c )

##############################
# Set handshake source files #
##############################

file(GLOB SOURCES "./*.c"
<<<<<<< HEAD
		$ENV{OCKAM_C_BASE}/source/ockam/handshake/xx/*.c )
=======
		$ENV{OCKAM_C_BASE}/source/ockam/handshake/*.c )
>>>>>>> 2a3b2b50c47508716005292b5f54c7c1048be7c3

###################################
# Set handshake_responder_test source files #
###################################

set(TEST_SRC ${TEST_SRC}
	${TEST_SRC_DIR}/handshake_responder_test.c
	${COMMON_OCKAM_SRC_DIR}/log/syslog.c)

set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}/${BINARY_DIR})

###########################
# Set the desired modules #
###########################

add_subdirectory(${TRANSPORT_POSIX_SRC_DIR} ${BINARY_DIR}/transport)
<<<<<<< HEAD
add_subdirectory(${XX_HANDSHAKE_SRC_DIR} ${BINARY_DIR}/handshake)
=======
add_subdirectory(${HANDSHAKE_SRC_DIR} ${BINARY_DIR}/handshake)
>>>>>>> 2a3b2b50c47508716005292b5f54c7c1048be7c3
add_subdirectory(${VAULT_SRC_DIR} ${BINARY_DIR}/vault)
add_subdirectory(${KAL_SRC_DIR} ${BINARY_DIR}/kal)
add_subdirectory(${LOG_SRC_DIR} ${BINARY_DIR}/log)
add_subdirectory(${MEM_SRC_DIR} ${BINARY_DIR}/mem)

###########################
# Set CLEAN options       #
###########################

option(ENABLE_DIRECTORY_CLEAN
  "Enable ADDITIONAL_CLEAN_FILES on directory level"
  on
)

#########################################
# Configure link libraries & executable #
#########################################

link_directories(${CMAKE_CURRENT_SOURCE_DIR}/${BINARY_DIR})
#	${CMAKE_CURRENT_SOURCE_DIR}/${BINARY_DIR}/vault
#	${CMAKE_CURRENT_SOURCE_DIR}/${BINARY_DIR}/transport
#	${CMAKE_CURRENT_SOURCE_DIR}/${BINARY_DIR}/handshake)

add_executable(handshake_responder_test ${TEST_SRC})

set_property(TARGET handshake_responder_test PROPERTY C_STANDARD 99)

target_link_libraries(handshake_responder_test ockam_transport)
target_link_libraries(handshake_responder_test ockam_handshake)
target_link_libraries(handshake_responder_test ockam_vault)
target_link_libraries(handshake_responder_test ockam_kal)
target_link_libraries(handshake_responder_test ockam_log)
target_link_libraries(handshake_responder_test ockam_mem)
target_link_libraries(handshake_responder_test mbedcrypto)

install(TARGETS handshake_responder_test DESTINATION bin)
