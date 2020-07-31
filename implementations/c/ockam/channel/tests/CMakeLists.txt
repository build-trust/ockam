
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

# ---
# okcam_channel_tests
# ---
add_executable(okcam_channel_tests
        channel_test.h
        channel_test.c
        initiator.c
        responder.c)

target_link_libraries(
    okcam_channel_tests
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

add_test(okcam_channel_tests okcam_channel_tests)
