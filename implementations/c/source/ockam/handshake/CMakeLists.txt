cmake_minimum_required(VERSION 3.13)
project(ockam_handshake C)



###########################
# Set directory locations #
###########################

set(COMMON_OCKAM_SRC_DIR $ENV{OCKAM_C_BASE}/source/ockam)
set(OCKAM_HANDSHAKE_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(HANDSHAKE_INC_DIR ${OCKAM_HANDSHAKE_SRC_DIR}/include)

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
