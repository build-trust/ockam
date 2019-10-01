cmake_minimum_required(VERSION 3.14)
project(ockam_vault)

###########################
# Set directory locations #
###########################

set(VAULT_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(OCKAM_DIR ${CMAKE_CURRENT_SOURCE_DIR}/..)
set(VAULT_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR}/src)
set(THIRD_PARTY_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../third-party/)

message( ${CMAKE_CURRENT_SOURCE_DIR} )

#################
# Build Options #
#################

set(CMAKE_VERBOSE_MAKEFILE TRUE)

# Vault Hardware Build Options
option(VAULT_HW_ATECC508A "Vault Hardware: Microchip ATECC508A" OFF)
option(VAULT_HW_ATECC608A "Vault Hardware: Microchip ATECC608A" ON)

# Vault Platform Build Options
option(VAULT_HAL_LINUX "Vault HAL: Linux" ON)
option(VAULT_HAL_FREERTOS "Vault HAL: FreeRTOS" OFF)

# Vault Interface Options
option(VAULT_IFACE_I2C "Vault IFace: I2C" ON)
option(VAULT_IFACE_SPI "Vault IFace: SPI" OFF)
option(VAULT_IFACE_UART "Vault IFace: UART" OFF)


#################
# Build Defines #
#################

# Set the base include directories
set(VAULT_INC ${OCKAM_DIR}/)

# Clear the Vault src val
set(VAULT_SRC)
set(VAULT_LIB)


########################
# Device Specific Code #
########################

# ATECC508A Specific Config 
if(VAULT_HW_ATECC508A)
set(VAULT_SRC ${VAULT_SRC} ${VAULT_SRC_DIR}/microchip/vault_atecc508a.c)
set(VAULT_HW_VENDOR_MICROCHIP TRUE)
endif()

# ATECC608A Specific Config 
if(VAULT_HW_ATECC608A)
set(VAULT_SRC ${VAULT_SRC} ${VAULT_SRC_DIR}/microchip/vault_atecc608a.c)
set(VAULT_HW_VENDOR_MICROCHIP TRUE)
endif()


########################
# Vendor Specific Code #
########################

# For Microchip Hardware, add cryptoauthlib
if(VAULT_HW_VENDOR_MICROCHIP)
set(VAULT_INC ${VAULT_INC} ${THIRD_PARTY_DIR}/microchip/)
set(VAULT_INC ${VAULT_INC} ${THIRD_PARTY_DIR}/microchip/cryptoauthlib/lib)
set(VAULT_INC ${VAULT_INC} ${THIRD_PARTY_DIR}/microchip/cryptoauthlib/lib/hal)
endif()


####################
# HAL Library Code #
####################

if(VAULT_HAL_LINUX)
add_definitions(-DVAULT_HAL_LINUX)
set(VAULT_SRC ${VAULT_SRC} ${VAULT_DIR}/hal/ockam_vault_hal_linux.c)
endif()

if(VAULT_HAL_FREERTOS)
add_definitions(-DVAULT_HAL_FREERTOS)
set(VAULT_SRC ${VAULT_SRC} ${VAULT_DIR}/hal/ockam_vault_hal_freertos.c)
endif()


####################
# Interface Option #
####################

if(VAULT_IFACE_I2C)
add_definitions(-DVAULT_IFACE_I2C)
endif()

if(VAULT_IFACE_SPI)
add_definitions(-DVAULT_IFACE_SPI)
endif()

if(VAULT_IFACE_UART)
add_definitions(-DVAULT_IFACE_UART)
endif()


###################
# Generate Build! #
###################


# Add the source files
add_library(ockam_vault ${VAULT_SRC})

# Set the include directories
#target_include_directories(ockam_vault PUBLIC $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}> PRIVATE ${VAULT_INC})
target_include_directories(ockam_vault PUBLIC ${VAULT_INC})

# Set the target
set_property(TARGET ockam_vault PROPERTY C_STANDARD 99)

# Add any extra libs
target_link_libraries(ockam_vault ${VAULT_LIBS})
