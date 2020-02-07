cmake_minimum_required(VERSION 3.13)
project(ockam_transport C)

# Ockam Transport

###########################
# Set directory locations #
###########################

set(COMMON_OCKAM_SRC_DIR $ENV{OCKAM_C_BASE}/source/ockam)
set(TRANSPORT_POSIX_SRC_DIR ${CMAKE_CURRENT_SOURCE_DIR})
set(OCKAM_INC_DIR ${OCKAM_C_BASE}/include)
set(TRANSPORT_INC_DIR ${OCKAM_INC_DIR}
  ${CMAKE_CURRENT_SOURCE_DIR}/include)


#################
# Build Options #
#################

# Unused for now
option(TRANSPORT_POSIX "Transport: POSIX" 1)

###################
# Generate Build! #
###################

# Add the source files
add_library(ockam_transport STATIC
  ${TRANSPORT_POSIX_SRC_DIR}/transport.c
  ${TRANSPORT_POSIX_SRC_DIR}/connection/socket/posix/posix_socket.c
  ${TRANSPORT_POSIX_SRC_DIR}/connection/socket/posix/posix_tcp_connection.c
  ${COMMON_OCKAM_SRC_DIR}/log/syslog.c)

# Set the include directories
target_include_directories(ockam_transport PUBLIC ${TRANSPORT_INC_DIR})

# Set the target
set_property(TARGET ockam_transport PROPERTY C_STANDARD 99)
