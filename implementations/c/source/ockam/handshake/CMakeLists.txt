cmake_minimum_required(VERSION 3.13)
project(ockam_handshake C)

# Ockam Transport

###########################
# Set directory locations #
###########################

set(COMMON_OCKAM_SRC_DIR $ENV{OCKAM_C_BASE}/source/ockam)
set(OCKAM_HANDSHAKE_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(OCKAM_INC_DIR ${OCKAM_C_BASE}/include)

#################
# Build Options #
#################

###################
# Generate Build! #
###################

# Add the source files
add_library(ockam_handshake STATIC
  ${OCKAM_HANDSHAKE_SRC_DIR}/handshake.c
  ${COMMON_OCKAM_SRC_DIR}/log/syslog.c)

# Set the include directories

# Set the target
set_property(TARGET ockam_handshake PROPERTY C_STANDARD 99)
