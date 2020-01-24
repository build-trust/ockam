cmake_minimum_required(VERSION 3.13)

###########################
# Path & Compiler Options #
###########################

# Always load the path.cmake file FIRST
include($ENV{OCKAM_C_BASE}/tools/cmake/path.cmake)

# This must be included BEFORE the project declaration
#!!include(${OCKAM_C_BASE}/tools/cmake/toolchains/raspberry-pi.cmake)
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

project(test_mbedcrypto)


###########################
# Set directory locations #
###########################

set(TEST_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR}/source)
set(TEST_CFG_DIR ${CMAKE_CURRENT_SOURCE_DIR}/config)

set(TEST_COMMON_SRC_DIR ${OCKAM_C_BASE}/test/ockam/vault/source)
set(TEST_COMMON_INC_DIR ${OCKAM_C_BASE}/test/ockam/vault/include)

set(OCKAM_SRC_DIR ${OCKAM_C_BASE}/source/ockam)
set(OCKAM_INC_DIR ${OCKAM_C_BASE}/include)

set(VAULT_SRC_DIR ${OCKAM_SRC_DIR}/vault)
set(KAL_SRC_DIR ${OCKAM_SRC_DIR}/kal)
set(LOG_SRC_DIR ${OCKAM_SRC_DIR}/log)
set(MEM_SRC_DIR ${OCKAM_SRC_DIR}/memory)

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
set(CMAKE_VERBOSE_MAKEFILE TRUE)


###########################
# Set include directories #
###########################

set(TEST_INC ${TEST_INC} ${OCKAM_INC_DIR})
set(TEST_INC ${TEST_INC} ${TEST_COMMON_INC_DIR})

include_directories(${TEST_INC} "./")


####################
# Set config files #
####################

add_definitions(-DOCKAM_VAULT_CONFIG_FILE="${TEST_CFG_DIR}/vault_config.h")
add_definitions(-DMBEDTLS_CONFIG_FILE="${TEST_CFG_DIR}/mbed_crypto_config.h")

####################
# Set source files #
####################

set(TEST_SRC ${TEST_SRC} ${TEST_SRC_DIR}/test_mbedcrypto.c)
set(TEST_SRC ${TEST_SRC} ${TEST_COMMON_SRC_DIR}/aes_gcm.c)
set(TEST_SRC ${TEST_SRC} ${TEST_COMMON_SRC_DIR}/hkdf.c)
set(TEST_SRC ${TEST_SRC} ${TEST_COMMON_SRC_DIR}/key_ecdh.c)
set(TEST_SRC ${TEST_SRC} ${TEST_COMMON_SRC_DIR}/print.c)
set(TEST_SRC ${TEST_SRC} ${TEST_COMMON_SRC_DIR}/random.c)
set(TEST_SRC ${TEST_SRC} ${TEST_COMMON_SRC_DIR}/sha256.c)

###########################
# Set the desired modules #
###########################

add_subdirectory(${VAULT_SRC_DIR} vault)
add_subdirectory(${KAL_SRC_DIR} kal)
add_subdirectory(${LOG_SRC_DIR} log)
add_subdirectory(${MEM_SRC_DIR} mem)

#########################################
# Configure link libraries & executable #
#########################################

link_directories(${CMAKE_ARCHIVE_OUTPUT_DIRECTORY})
add_executable(test_mbedcrypto ${TEST_SRC})

target_link_libraries(test_mbedcrypto ockam_vault)
target_link_libraries(test_mbedcrypto ockam_kal)
target_link_libraries(test_mbedcrypto ockam_log)
target_link_libraries(test_mbedcrypto ockam_mem)
target_link_libraries(test_mbedcrypto mbedcrypto)

install(TARGETS test_mbedcrypto DESTINATION bin)
