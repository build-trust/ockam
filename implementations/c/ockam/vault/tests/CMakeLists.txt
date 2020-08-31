
if(NOT BUILD_TESTING)
    return()
endif()

find_package(cmocka QUIET)
if(NOT cmocka_FOUND)
    return()
endif()

# ---
# ockam_vault_tests
# ---
add_library(ockam_vault_tests)

target_include_directories(ockam_vault_tests
    PUBLIC
        ${CMAKE_CURRENT_LIST_DIR})

target_sources(ockam_vault_tests
    PRIVATE
        aead_aes_gcm.c
        hkdf.c
        hkdf_aead.c
        random.c
        secret_ecdh.c
        sha256.c
        test_vault.h)

target_link_libraries(ockam_vault_tests
    PUBLIC
        ockam_vault_interface
        ockam_error_interface
        ockam_memory_interface
        cmocka-static
)
