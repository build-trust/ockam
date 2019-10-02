cmake_minimum_required(VERSION 3.13)

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

