
if(NOT BUILD_TESTING)
    return()
endif()

if (WIN32)
  return()
endif()

find_package(cmocka QUIET)
if(NOT cmocka_FOUND)
    return()
endif()

configure_file(${CMAKE_CURRENT_SOURCE_DIR}/route.txt
        ${CMAKE_BINARY_DIR}/ockam/channel/tests/route.txt)

# ---
# ockam_channel_tests
# ---
add_executable(ockam_channel_initiator_test
        channel_test.h
    initiator.c
        read_configuration.c)

target_link_libraries(
        ockam_channel_initiator_test
    PUBLIC
        ockam::key_agreement_interface
        ockam::vault_default
        ockam::random_urandom
        ockam::memory_stdlib
        ockam::log
        ockam::transport_posix_socket
        ockam::channel
        cmocka-static
)

# FIXME
#add_test(ockam_channel_tests ockam_channel_initiator_test)

add_executable(ockam_channel_responder_test
        channel_test.h
    responder.c
        read_configuration.c)

target_link_libraries(
        ockam_channel_responder_test
        PUBLIC
        ockam::key_agreement_interface
        ockam::vault_default
        ockam::random_urandom
        ockam::memory_stdlib
        ockam::log
        ockam::transport_posix_socket
        ockam::channel
        cmocka-static
)

# FIXME
#add_test(ockam_channel_tests ockam_channel_responder_test)
