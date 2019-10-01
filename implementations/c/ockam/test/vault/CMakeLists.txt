cmake_minimum_required(VERSION 3.14)
project(test_atecc608a)


###########################
# Set directory locations #
###########################

set(TEST_DIR ${CMAKE_SOURCE_DIR})
set(VAULT_DIR ${TEST_DIR}/../../vault/)
set(OCKAM_DIR ${TEST_DIR}/../../)
set(THIRD_PARTY_DIR ${TEST_DIR}/../../third-party/)

set(LIB_DIR ${VAULT_DIR}/_build ${THIRD_PARTY_DIR}/microchip/cryptoauthlib/lib/)

include_directories(${OCKAM_DIR})
link_directories(${LIB_DIR})

add_executable(test_atecc608a test_atecc608a.c)
target_link_libraries(test_atecc608a ockam_vault)
target_link_libraries(test_atecc608a cryptoauth)

install(TARGETS test_atecc608a DESTINATION bin)
