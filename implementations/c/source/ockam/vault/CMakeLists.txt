cmake_minimum_required(VERSION 3.13)
project(ockam_vault)

# Clear the Vault vars 
set(VAULT_INC)
set(VAULT_SRC)
set(CMAKE_VERBOSE_MAKEFILE TRUE)


#######################
# Directory Locations #
#######################

set(VAULT_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(VAULT_TPM_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR}/tpm)
set(VAULT_HOST_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR}/host)

set(OCKAM_INC_DIR ${OCKAM_C_BASE}/include)
set(THIRD_PARTY_DIR ${OCKAM_C_BASE}/third-party/)


#################
# Build Options #
#################

# Vault External Build Options
option(VAULT_TPM_ATECC508A "Vault TPM: Microchip ATECC508A")
option(VAULT_TPM_ATECC608A "Vault TPM: Microchip ATECC608A")
option(VAULT_TPM_SE050 "Vault TPM: NXP SE050")

# Vault Software Build Options
option(VAULT_HOST_OCKAM "Vault Host: Ockam")
option(VAULT_HOST_MBEDCRYPTO "Vault Host: mbed Crypto")
option(VAULT_HOST_OPENSSL "Vault Host: OpenSSL")

# Vault External Interface Options
option(VAULT_TPM_IFACE_I2C "Vault IFace: I2C")
option(VAULT_TPM_IFACE_SPI "Vault IFace: SPI")
option(VAULT_TPM_IFACE_UART "Vault IFace: UART")


###################
# Vault Interface #
###################

set(VAULT_SRC ${VAULT_SRC_DIR}/vault.c)
set(VAULT_INC ${OCKAM_INC_DIR})


##########################
# External Specific Code #
##########################

# ATECC508A Specific Config 
if(VAULT_TPM_ATECC508A)
    set(VAULT_SRC ${VAULT_SRC} ${VAULT_TPM_SRC_DIR}/microchip/atecc508a.c)
    include(${OCKAM_C_BASE}/tools/cmake/third-party/microchip.cmake)
endif()

# ATECC608A Specific Config 
if(VAULT_TPM_ATECC608A)
    set(VAULT_SRC ${VAULT_SRC} ${VAULT_TPM_SRC_DIR}/microchip/atecc608a.c)
    include(${OCKAM_C_BASE}/tools/cmake/third-party/microchip.cmake)
endif()


######################
# Host Specific Code #
######################

# Ockam Host Code
if(VAULT_HOST_OCKAM)
    set(VAULT_SRC ${VAULT_SRC} ${VAULT_HOST_SRC_DIR}/ockam.c)
endif()

# mbed crypto Host Code
if(VAULT_HOST_MBEDCRYPTO)
    set(VAULT_SRC ${VAULT_SRC} ${VAULT_HOST_SRC_DIR}/mbedcrypto.c)
    include(${OCKAM_C_BASE}/tools/cmake/third-party/mbedcrypto.cmake)
endif()

# OpenSSL Config
if(VAULT_HOST_OpenSSL)
    set(VAULT_SRC ${VAULT_SRC} ${VAULT_HOST_SRC_DIR}/openssl.c)
    include(${OCKAM_C_CMAKE_THIRD_PARTY}/openssl.cmake)
endif()


###################
# Generate Build! #
###################

# Add the source files
add_library(ockam_vault ${VAULT_SRC})

# Set the include directories
target_include_directories(ockam_vault PUBLIC ${VAULT_INC})

# Set the target
set_property(TARGET ockam_vault PROPERTY C_STANDARD 99)
