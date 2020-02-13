cmake_minimum_required(VERSION 3.14)

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

project(handshake_initiator_demo)


###########################
# Set directory locations #
###########################

set(DEMO_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(DEMO_CFG_DIR ${CMAKE_CURRENT_SOURCE_DIR}/config)

set(DEMO_INC_DIR ${CMAKE_CURRENT_SOURCE_DIR}/../include)

set(OCKAM_SRC_DIR ${OCKAM_C_BASE}/source/ockam)
set(OCKAM_INC_DIR ${OCKAM_C_BASE}/include)

set(VAULT_SRC_DIR ${OCKAM_SRC_DIR}/vault)
set(KAL_SRC_DIR ${OCKAM_SRC_DIR}/kal)
set(LOG_SRC_DIR ${OCKAM_SRC_DIR}/log)
set(MEM_SRC_DIR ${OCKAM_SRC_DIR}/memory)
set(TRANSPORT_SRC_DIR ${OCKAM_SRC_DIR}/transport)
set(HANDSHAKE_SRC_DIR ${OCKAM_SRC_DIR}/handshake)

set(THIRD_PARTY_DIR ${OCKAM_C_BASE}/third-party)

set(BINARY_DIR _build/$ENV{PLATFORM})

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

set(DEMO_INC ${DEMO_INC} ${OCKAM_INC_DIR})
set(DEMO_INC ${DEMO_INC} ${DEMO_INC_DIR})

include_directories(${DEMO_INC})

####################
# Set config files #
####################

add_definitions(-DOCKAM_VAULT_CONFIG_FILE="${DEMO_SRC_DIR}/config/vault_config.h")

####################
# Set source files #
####################

set(DEMO_SRC ${CMAKE_CURRENT_SOURCE_DIR}/handshake_initiator_demo.c)

###########################
# Set the desired modules #
###########################

add_subdirectory(${VAULT_SRC_DIR} vault)
add_subdirectory(${KAL_SRC_DIR} kal)
add_subdirectory(${LOG_SRC_DIR} log)
add_subdirectory(${MEM_SRC_DIR} mem)
add_subdirectory(${TRANSPORT_SRC_DIR} transport)
add_subdirectory(${HANDSHAKE_SRC_DIR} handshake)

#########################################
# Configure link libraries & executable #
#########################################

link_directories(${CMAKE_CURRENT_SOURCE_DIR}/${BINARY_DIR})
add_executable(handshake_initiator_demo ${DEMO_SRC})

target_link_libraries(handshake_initiator_demo ockam_vault)
target_link_libraries(handshake_initiator_demo ockam_kal)
target_link_libraries(handshake_initiator_demo ockam_log)
target_link_libraries(handshake_initiator_demo mbedcrypto)
target_link_libraries(handshake_initiator_demo ockam_transport)
target_link_libraries(handshake_initiator_demo ockam_handshake)

install(TARGETS handshake_initiator_demo DESTINATION bin)
