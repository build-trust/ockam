
# ---
# ockam_vault_rust_default_tests
# ---
add_executable(ockam_vault_rust_default_tests vault.c)

target_link_libraries(ockam_vault_rust_default_tests
  PRIVATE
    ockam::vault_rust_interface
    ockam::vault_rust_default
)

add_test(ockam_vault_rust_default_tests ockam_vault_rust_default_tests)
