
# ---
# ockam::transport_posix_socket
# ---
add_library(ockam_transport_posix_socket)
add_library(ockam::transport_posix_socket ALIAS ockam_transport_posix_socket)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)
target_include_directories(ockam_transport_posix_socket PUBLIC ${INCLUDE_DIR})

file(COPY socket.h DESTINATION ${INCLUDE_DIR}/ockam/transport/)
#file(COPY socket_tcp.h DESTINATION ${INCLUDE_DIR}/ockam/transport/)
file(COPY socket_udp.h DESTINATION ${INCLUDE_DIR}/ockam/transport/)

target_sources(ockam_transport_posix_socket
    PRIVATE
        socket.c
#        socket_tcp.c
        socket_udp.c
    PUBLIC
        ${INCLUDE_DIR}/ockam/transport/socket.h
#        ${INCLUDE_DIR}/ockam/transport/socket_tcp.h
        ${INCLUDE_DIR}/ockam/transport/socket_udp.h
)

target_link_libraries(ockam_transport_posix_socket
    PUBLIC
        ockam::transport
        ockam::io
    PRIVATE
        ockam::log)

add_subdirectory(tests)
