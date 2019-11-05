cmake_minimum_required(VERSION 3.13)
project(test_atecc508a)

#set(CMAKE_VERBOSE_MAKEFILE TRUE)

###########################
# Set directory locations #
###########################

set(TEST_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(OCKAM_DIR ${TEST_DIR}/../../../ockam)
set(VAULT_DIR ${OCKAM_DIR}/vault)
set(COMMON_DIR ${OCKAM_DIR}/common)
set(THIRD_PARTY_DIR ${OCKAM_DIR}/third-party)

set(LIB_DIR ${VAULT_DIR}/ ${THIRD_PARTY_DIR}/microchip/cryptoauthlib/lib/ ${THIRD_PARTY_DIR}/arm/mbedtls/)


###########################
# Set include directories #
###########################

set(TEST_INC cfg)
set(TEST_INC ${TEST_INC} ${OCKAM_DIR})
set(TEST_INC ${TEST_INC} ${OCKAM_DIR}/vault/src/hw/microchip)
set(TEST_INC ${TEST_INC} ${THIRD_PARTY_DIR}/microchip)
set(TEST_INC ${TEST_INC} ${THIRD_PARTY_DIR}/microchip/cryptoauthlib/lib)
set(TEST_INC ${TEST_INC} ${THIRD_PARTY_DIR}/microchip/cryptoauthlib/lib/hal)

include_directories(${TEST_INC})

add_subdirectory(${VAULT_DIR} vault)
add_subdirectory(${COMMON_DIR} common)
#add_subdirectory(${THIRD_PARTY_DIR}/arm/mbedtls mbedtls)
#add_subdirectory(${THIRD_PARTY_DIR}/microchip/cryptoauthlib cryptoauthlib)

link_directories(${CMAKE_ARCHIVE_OUTPUT_DIRECTORY})

add_executable(test_atecc508a src/test_atecc508a.c)
target_link_libraries(test_atecc508a ockam_vault)
target_link_libraries(test_atecc508a ockam_common)
target_link_libraries(test_atecc508a cryptoauth)
target_link_libraries(test_atecc508a mbedtls)

install(TARGETS test_atecc508a DESTINATION bin)
