
# ---
# ockam::vault_rust_interface
# ---
add_library(ockam_vault_rust_interface INTERFACE)
add_library(ockam::vault_rust_interface ALIAS ockam_vault_rust_interface)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)

target_include_directories(ockam_vault_rust_interface INTERFACE ${INCLUDE_DIR})

file(COPY vault.h DESTINATION ${INCLUDE_DIR}/ockam/rs/)

target_sources(
  ockam_vault_rust_interface
  INTERFACE
    ${INCLUDE_DIR}/ockam/rs/vault.h
)

# ---
# ockam::vault_rust_default
# ---
add_library(ockam_vault_rust_default STATIC IMPORTED GLOBAL)
add_library(ockam::vault_rust_default ALIAS ockam_vault_rust_default)

set_target_properties(ockam_vault_rust_default PROPERTIES IMPORTED_LOCATION
${CMAKE_CURRENT_LIST_DIR}/../../../../rs/target/release/libockam_vault.a)

add_subdirectory(tests)
