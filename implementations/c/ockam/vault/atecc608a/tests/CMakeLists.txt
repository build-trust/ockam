
if(NOT BUILD_TESTING)
    return()
endif()

find_package(cmocka QUIET)
if(NOT cmocka_FOUND)
    return()
endif()

# ---
# ockam_vault_atecc608a_tests
# ---
add_executable(ockam_vault_atecc608a_tests test_atecc608a.c)

target_link_libraries(ockam_vault_atecc608a_tests
    PUBLIC
        ockam::vault_interface
        ockam::vault_atecc608a
        ockam::memory_stdlib
        ockam::log
        ockam_vault_tests
        cmocka-static)

add_test(ockam_vault_atecc608a_tests ockam_vault_atecc608a_tests)
