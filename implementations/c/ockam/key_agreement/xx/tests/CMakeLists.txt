
if(NOT BUILD_TESTING)
    return()
endif()

find_package(cmocka QUIET)
if(NOT cmocka_FOUND)
    return()
endif()

# ---
# okcam_key_agreement_xx_tests
# ---
add_executable(okcam_key_agreement_xx_tests
        xx_test.c
        xx_test.h
        xx_test_initiator.c
        xx_test_responder.c)

target_link_libraries(okcam_key_agreement_xx_tests
    PUBLIC
        cmocka-static
        ockam::error_interface
        ockam::key_agreement
        ockam::io
        ockam::transport
        ockam::vault_default
        ockam::key_agreement_xx
        ockam::memory_stdlib
        ockam::random_urandom
        ockam::transport_posix_socket
)

add_test(okcam_key_agreement_xx_tests okcam_key_agreement_xx_tests)
