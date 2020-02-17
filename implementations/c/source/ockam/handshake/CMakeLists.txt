cmake_minimum_required(VERSION 3.14)
project(ockam_handshake C)



###########################
# Set directory locations #
###########################

set(COMMON_OCKAM_SRC_DIR $ENV{OCKAM_C_BASE}/source/ockam)
set(COMMON_OCKAM_INC_DIR ${OCKAM_C_BASE}/include/ockam)
set(OCKAM_HANDSHAKE_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(HANDSHAKE_INC_DIR ${COMMON_OCKAM_INC_DIR})
set(HANDSHAKE_INC_DIR ${HANDSHAKE_INC_DIR} ${COMMON_OCKAM_INC_DIR}/handshake)

#################
# Build Options #
#################

###################
# Generate Build! #
###################

# Add the source files
add_library(ockam_handshake STATIC
  ${OCKAM_HANDSHAKE_SRC_DIR}/xx/xx_handshake.c
  ${OCKAM_HANDSHAKE_SRC_DIR}/xx/xx_initiator.c
  ${OCKAM_HANDSHAKE_SRC_DIR}/xx/xx_responder.c
        ${COMMON_OCKAM_SRC_DIR}/log/syslog.c)

# Set the include directories
target_include_directories(ockam_handshake PUBLIC ${HANDSHAKE_INC_DIR})

# Set the target
set_property(TARGET ockam_handshake PROPERTY C_STANDARD 99)
