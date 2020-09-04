
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
# ockam_key_agreement_xx_tests
# ---
add_executable(ockam_key_agreement_xx_tests
        init_vault.c
        xx_test.c
        xx_test.h
        xx_test_initiator.c
        xx_test_responder.c)

target_link_libraries(ockam_key_agreement_xx_tests
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

if (OCKAM_ENABLE_ATECC608A_TESTS)
    target_compile_definitions(ockam_key_agreement_xx_tests PUBLIC OCKAM_ENABLE_ATECC608A_TESTS)
    target_link_libraries(ockam_key_agreement_xx_tests
        PUBLIC
            ockam::vault_atecc608a)
endif()

add_test(ockam_key_agreement_xx_tests ockam_key_agreement_xx_tests)
