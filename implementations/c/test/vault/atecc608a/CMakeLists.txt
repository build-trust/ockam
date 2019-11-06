cmake_minimum_required(VERSION 3.13)
project(test_atecc608a)

#################
# Build Options #
#################

# Vault Hardware Build Options
set(VAULT_HW_ATECC608A TRUE)
set(VAULT_IFACE_I2C TRUE)


###########################
# Set directory locations #
###########################

set(TEST_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(OCKAM_DIR ${TEST_DIR}/../../../ockam)
set(VAULT_DIR ${OCKAM_DIR}/vault)
set(COMMON_DIR ${OCKAM_DIR}/common)
set(THIRD_PARTY_DIR ${OCKAM_DIR}/third-party)

set(LIB_DIR ${VAULT_DIR}/ ${THIRD_PARTY_DIR}/microchip/cryptoauthlib/lib/)


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


###########################
# Set the desired modules #
###########################

add_subdirectory(${VAULT_DIR} vault)
add_subdirectory(${COMMON_DIR} common)


#########################################
# Configure link libraries & executable #
#########################################

link_directories(${CMAKE_ARCHIVE_OUTPUT_DIRECTORY})

add_executable(test_atecc608a src/test_atecc608a.c)
target_link_libraries(test_atecc608a ockam_vault)
target_link_libraries(test_atecc608a ockam_common)
target_link_libraries(test_atecc608a cryptoauth)

install(TARGETS test_atecc608a DESTINATION bin)
