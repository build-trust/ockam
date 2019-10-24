cmake_minimum_required(VERSION 3.13)
project(test_atecc508a)


###########################
# Set directory locations #
###########################

set(CMAKE_VERBOSE_MAKEFILE TRUE)

set(TEST_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(OCKAM_DIR ${TEST_DIR}/../../ockam/)
set(VAULT_DIR ${OCKAM_DIR}/vault/)
set(COMMON_DIR ${OCKAM_DIR}/common/)
set(THIRD_PARTY_DIR ${OCKAM_DIR}/third-party/)

set(LIB_DIR ${VAULT_DIR}/ ${THIRD_PARTY_DIR}/microchip/cryptoauthlib/lib/)

set(TEST_INC ${OCKAM_DIR})
set(TEST_INC ${TEST_INC} ${OCKAM_DIR}/vault/src/hw/microchip)
set(TEST_INC ${TEST_INC} ${THIRD_PARTY_DIR}/microchip)
set(TEST_INC ${TEST_INC} ${THIRD_PARTY_DIR}/microchip/cryptoauthlib/lib)
set(TEST_INC ${TEST_INC} ${THIRD_PARTY_DIR}/microchip/cryptoauthlib/lib/hal)

include_directories(${TEST_INC})

link_directories(${VAULT_DIR} ${COMMON_DIR} ${THIRD_PARTY_DIR}/microchip/cryptoauthlib/lib)

add_executable(test_atecc508a test_atecc508a.c)
target_link_libraries(test_atecc508a ockam_vault)
target_link_libraries(test_atecc508a ockam_common)
target_link_libraries(test_atecc508a cryptoauth)

install(TARGETS test_atecc508a DESTINATION bin)
