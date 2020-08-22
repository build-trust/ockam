
if(NOT BUILD_TESTING)
    return()
endif()

find_package(cmocka QUIET)
if(NOT cmocka_FOUND)
    return()
endif()

set(COMMON_SRCS
        client.c
        client.h
        runner.c
        runner.h
        server.c
        server.h
        tools.c
        tools.h)

set(COMMON_DEPENDENCIES
        ockam_log
        ockam_memory_stdlib
        ockam_io
        ockam_transport
        ockam_transport_posix_socket)

file(COPY fixtures/client_test_data.txt DESTINATION ${CMAKE_CURRENT_BINARY_DIR}/fixtures/)
file(COPY fixtures/server_test_data.txt DESTINATION ${CMAKE_CURRENT_BINARY_DIR}/fixtures/)

add_executable(ockam_transport_posix_udp_test)
target_sources(ockam_transport_posix_udp_test
    PUBLIC
        ${COMMON_SRCS}
        udp_test.c)
target_link_libraries(ockam_transport_posix_udp_test
    PRIVATE
        ${COMMON_DEPENDENCIES}
    PUBLIC
        cmocka)
add_test(ockam_transport_posix_udp_test ockam_transport_posix_udp_test)

#add_executable(ockam_transport_posix_tcp_test)
#target_sources(ockam_transport_posix_tcp_test
#    PUBLIC
#        ${COMMON_SRCS}
#        tcp_test.c)
#target_link_libraries(ockam_transport_posix_tcp_test
#    PRIVATE
#        ${COMMON_DEPENDENCIES}
#    PUBLIC
#        cmocka)
#add_test(ockam_transport_posix_tcp_test ockam_transport_posix_tcp_test)
