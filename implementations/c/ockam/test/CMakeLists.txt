cmake_minimum_required(VERSION 3.14)

#################
# Build Options #
#################

option(OCKAM_TEST_VAULT "Build Test for Vault" ON)


###############
# Ockam Tests #
###############

if(OCKAM_TEST_VAULT)
add_subdirectory("vault/")
endif()

