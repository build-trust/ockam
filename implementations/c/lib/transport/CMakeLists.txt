# Ockam Transport

project(transport C)

message(STATUS "------------------ ockam_transport ---------------")
ockam_cc_library(
  NAME
    transport
  SRCS
        transport_impl.c
        transport_impl.h
    posix_socket/socket.c
        posix_socket/socket.h
        posix_socket/socket_tcp.c
        posix_socket/socket_tcp.h
    PUBLIC )

add_subdirectory(posix_socket)