
# ---
# ockam::ex_interface
# ---
add_library(ockam_kex_interface INTERFACE)
add_library(ockam::kex_interface ALIAS ockam_kex_interface)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)

target_include_directories(ockam_kex_interface INTERFACE ${INCLUDE_DIR})

file(COPY ../../../../../../rust/kex/include/kex.h DESTINATION ${INCLUDE_DIR}/ockam)

target_sources(
  ockam_kex_interface
  INTERFACE
    ${INCLUDE_DIR}/ockam/kex.h
)

# ---
# ockam::kex_rust
# ---
add_library(ockam_kex SHARED IMPORTED GLOBAL)
add_library(ockam::kex ALIAS ockam_kex)

set(kex_dylib_path ../../../../../../rust/target/debug/libockam_kex.dylib)
get_filename_component(real_kex_dylib_path "${kex_dylib_path}" REALPATH BASE_DIR "${CMAKE_CURRENT_LIST_DIR}")
set_target_properties(
  ockam_kex
  PROPERTIES
    IMPORTED_LOCATION "${real_kex_dylib_path}"
)

# ---
# ockam::elixir_kex_rust
# ---
add_library(ockam_elixir_kex_rust)
add_library(ockam::elixir_kex_rust ALIAS ockam_elixir_kex_rust)

target_sources(ockam_elixir_kex_rust PRIVATE nifs.c)

target_include_directories(ockam_elixir_kex_rust PUBLIC $ENV{ERL_INCLUDE_DIR})
set_target_properties(ockam_elixir_kex_rust PROPERTIES LINK_FLAGS "-dynamiclib -undefined dynamic_lookup")

target_link_libraries(
  ockam_elixir_kex_rust
  PUBLIC
    ockam::kex_interface
    ockam::kex
)
