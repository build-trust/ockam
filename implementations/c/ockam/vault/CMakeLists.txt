
# ---
# ockam::vault_interface
# ---
add_library(ockam_vault_interface INTERFACE)
add_library(ockam::vault_interface ALIAS ockam_vault_interface)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)

target_include_directories(ockam_vault_interface INTERFACE ${INCLUDE_DIR})

file(COPY vault.h DESTINATION ${INCLUDE_DIR}/ockam)
file(COPY impl.h DESTINATION ${INCLUDE_DIR}/ockam/vault)

target_sources(
  ockam_vault_interface
  INTERFACE
    ${INCLUDE_DIR}/ockam/vault.h
    ${INCLUDE_DIR}/ockam/vault/impl.h
)

# ---
# ockam::vault
# ---
add_library(ockam_vault)
add_library(ockam::vault ALIAS ockam_vault)

target_sources(
  ockam_vault
  PRIVATE
    vault.c
)

target_link_libraries(
  ockam_vault
  PUBLIC
    ockam::log
    ockam::error_interface
    ockam::memory_interface
    ockam::vault_interface
)

add_subdirectory(tests)