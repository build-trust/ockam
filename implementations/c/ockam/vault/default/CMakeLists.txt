
find_package(bearssl REQUIRED)

# ---
# ockam::vault_default
# ---
add_library(ockam_vault_default)
add_library(ockam::vault_default ALIAS ockam_vault_default)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)
target_include_directories(ockam_vault_default PUBLIC ${INCLUDE_DIR})

file(COPY default.h DESTINATION ${INCLUDE_DIR}/ockam/vault/)
target_sources(
  ockam_vault_default
  PRIVATE
    default.c
  PUBLIC
    ${INCLUDE_DIR}/ockam/vault/default.h
)

target_link_libraries(
  ockam_vault_default
  PRIVATE
    bearssl
    ockam::memory_interface
    ockam::error_interface
    ockam::vault
  PUBLIC
    ockam::random_interface
    ockam::vault_interface
)

add_subdirectory(tests)
