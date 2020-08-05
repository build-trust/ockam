
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
# ockam_vault_default_tests
# ---
add_executable(ockam_vault_default_tests test_default.c)

target_link_libraries(ockam_vault_default_tests
    PUBLIC
        ockam::vault_interface
        ockam::vault_default
        ockam::random_interface
        ockam::memory_stdlib
        ockam::random_urandom
        ockam::log
        ockam_vault_tests
        cmocka-static
)

add_test(ockam_vault_default_tests ockam_vault_default_tests)
