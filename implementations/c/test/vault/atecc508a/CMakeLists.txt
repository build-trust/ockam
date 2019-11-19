cmake_minimum_required(VERSION 3.13)

###########################
# Path & Compiler Options #
###########################

include($ENV{OCKAM_C_BASE}/tools/cmake/path.cmake)
include(${OCKAM_C_BASE}/tools/cmake/toolchains/raspberry-pi.cmake)

###########
# Project #
###########

project(test_atecc508a)


###########################
# Set directory locations #
###########################

set(TEST_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR}/source)
set(TEST_CFG_DIR ${CMAKE_CURRENT_SOURCE_DIR}/config)

set(OCKAM_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../../..)
set(OCKAM_SRC_DIR ${OCKAM_DIR}/source)
set(OCKAM_INC_DIR ${OCKAM_DIR}/include)

set(VAULT_SRC_DIR ${OCKAM_SRC_DIR}/vault)
set(KAL_SRC_DIR ${OCKAM_SRC_DIR}/kal)
set(LOG_SRC_DIR ${OCKAM_SRC_DIR}/log)
set(MEM_SRC_DIR ${OCKAM_SRC_DIR}/mem)

set(THIRD_PARTY_DIR ${OCKAM_DIR}/third-party)


#################
# Build Options #
#################

# Vault Hardware Build Options
set(VAULT_HW_ATECC508A TRUE)
set(VAULT_SW_MBEDTLS TRUE)
set(VAULT_IFACE_I2C TRUE)

# KAL Build Option
set(KAL_LINUX TRUE)

# Log Build Option
set(LOG_LINUX TRUE)

# Mem Build Option
set(MEM_STDLIB TRUE)

# Compiler Build Options
set(CMAKE_VERBOSE_MAKEFILE TRUE)


###########################
# Set include directories #
###########################

set(TEST_INC ${TEST_CFG_DIR})
set(TEST_INC ${TEST_INC} ${OCKAM_INC_DIR})
set(TEST_INC ${TEST_INC} ${THIRD_PARTY_DIR}/microchip)
set(TEST_INC ${TEST_INC} ${THIRD_PARTY_DIR}/microchip/cryptoauthlib/lib)
set(TEST_INC ${TEST_INC} ${THIRD_PARTY_DIR}/microchip/cryptoauthlib/lib/hal)

include_directories(${TEST_INC})


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

add_executable(test_atecc508a ${TEST_SRC_DIR}/test_atecc508a.c)

target_link_libraries(test_atecc508a ockam_vault)
target_link_libraries(test_atecc508a ockam_kal)
target_link_libraries(test_atecc508a ockam_log)
target_link_libraries(test_atecc508a ockam_mem)
target_link_libraries(test_atecc508a cryptoauth)
target_link_libraries(test_atecc508a mbedtls)

install(TARGETS test_atecc508a DESTINATION bin)
