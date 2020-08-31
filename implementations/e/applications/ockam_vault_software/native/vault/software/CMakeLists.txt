
# ---
# ockam::vault_interface
# ---
add_library(ockam_vault_interface INTERFACE)
add_library(ockam::vault_interface ALIAS ockam_vault_interface)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)

target_include_directories(ockam_vault_interface INTERFACE ${INCLUDE_DIR})

file(COPY ../../../../../../rust/vault/include/vault.h DESTINATION ${INCLUDE_DIR}/ockam)

target_sources(
  ockam_vault_interface
  INTERFACE
    ${INCLUDE_DIR}/ockam/vault.h
)

# ---
# ockam::vault_rust_default
# ---
add_library(ockam_vault SHARED IMPORTED GLOBAL)
add_library(ockam::vault ALIAS ockam_vault)

set(vault_dylib_path ../../../../../../rust/target/debug/libockam_vault.dylib)
get_filename_component(real_vault_dylib_path "${vault_dylib_path}" REALPATH BASE_DIR "${CMAKE_CURRENT_LIST_DIR}")
set_target_properties(
  ockam_vault
  PROPERTIES
    IMPORTED_LOCATION "${real_vault_dylib_path}"
)

# ---
# ockam::elixir_vault_software
# ---
add_library(ockam_elixir_vault_software)
add_library(ockam::elixir_vault_software ALIAS ockam_elixir_vault_software)

target_sources(ockam_elixir_vault_software PRIVATE nifs.c)

target_include_directories(ockam_elixir_vault_software PUBLIC $ENV{ERL_INCLUDE_DIR})
set_target_properties(ockam_elixir_vault_software PROPERTIES LINK_FLAGS "-dynamiclib -undefined dynamic_lookup")

target_link_libraries(
  ockam_elixir_vault_software
  PUBLIC
    ockam::vault_interface
    ockam::vault
)
