
# ---
# ockam::vault_interface
# ---
add_library(ockam_vault_interface INTERFACE)
add_library(ockam::vault_interface ALIAS ockam_vault_interface)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)

target_include_directories(ockam_vault_interface INTERFACE ${INCLUDE_DIR})

file(COPY ../../../../../../rust/vault/ffi/include/vault.h DESTINATION ${INCLUDE_DIR}/ockam)

target_sources(
  ockam_vault_interface
  INTERFACE
    ${INCLUDE_DIR}/ockam/vault.h
)

# ---
# ockam::vault_rust_default
# ---

add_library(ockam_vault_ffi SHARED IMPORTED GLOBAL)
add_library(ockam::vault_ffi ALIAS ockam_vault_ffi)

add_library(ockam_vault_file SHARED IMPORTED GLOBAL)
add_library(ockam::vault_file ALIAS ockam_vault_file)

add_library(ockam_vault_software SHARED IMPORTED GLOBAL)
add_library(ockam::vault_software ALIAS ockam_vault_software)

if(APPLE)
file(COPY ../../../../../../rust/target/debug/libockam_vault_ffi.dylib DESTINATION "${CMAKE_CURRENT_BINARY_DIR}")
set(vault_ffi_lib_path "${CMAKE_CURRENT_BINARY_DIR}/libockam_vault_ffi.dylib")
file(COPY ../../../../../../rust/target/debug/libockam_vault_file.dylib DESTINATION "${CMAKE_CURRENT_BINARY_DIR}")
set(vault_file_lib_path "${CMAKE_CURRENT_BINARY_DIR}/libockam_vault_file.dylib")
file(COPY ../../../../../../rust/target/debug/libockam_vault_software.dylib DESTINATION "${CMAKE_CURRENT_BINARY_DIR}")
set(vault_software_lib_path "${CMAKE_CURRENT_BINARY_DIR}/libockam_vault_software.dylib")
endif()

if(UNIX AND NOT APPLE)
file(COPY ../../../../../../rust/target/debug/libockam_vault_ffi.so DESTINATION "${CMAKE_CURRENT_BINARY_DIR}")
set(vault_ffi_lib_path "${CMAKE_CURRENT_BINARY_DIR}/libockam_vault_ffi.so")
file(COPY ../../../../../../rust/target/debug/libockam_vault_file.so DESTINATION "${CMAKE_CURRENT_BINARY_DIR}")
set(vault_file_lib_path "${CMAKE_CURRENT_BINARY_DIR}/libockam_vault_file.so")
file(COPY ../../../../../../rust/target/debug/libockam_vault_software.so DESTINATION "${CMAKE_CURRENT_BINARY_DIR}")
set(vault_software_lib_path "${CMAKE_CURRENT_BINARY_DIR}/libockam_vault_software.so")
endif()

get_filename_component(real_vault_ffi_lib_path "${vault_ffi_lib_path}" REALPATH BASE_DIR "${CMAKE_CURRENT_BINARY_DIR}")
set_target_properties(
        ockam_vault_ffi
        PROPERTIES
        IMPORTED_LOCATION "${real_vault_ffi_lib_path}"
)

get_filename_component(real_vault_file_lib_path "${vault_file_lib_path}" REALPATH BASE_DIR "${CMAKE_CURRENT_BINARY_DIR}")
set_target_properties(
        ockam_vault_file
        PROPERTIES
        IMPORTED_LOCATION "${real_vault_file_lib_path}"
)

get_filename_component(real_vault_software_lib_path "${vault_software_lib_path}" REALPATH BASE_DIR "${CMAKE_CURRENT_BINARY_DIR}")
set_target_properties(
        ockam_vault_software
        PROPERTIES
        IMPORTED_LOCATION "${real_vault_software_lib_path}"
)

# ---
# ockam::elixir_vault_software
# ---
add_library(ockam_elixir_vault_software)
add_library(ockam::elixir_vault_software ALIAS ockam_elixir_vault_software)

target_sources(ockam_elixir_vault_software PRIVATE nifs.c)

target_include_directories(ockam_elixir_vault_software PUBLIC $ENV{ERL_INCLUDE_DIR})

if(APPLE)
set_target_properties(ockam_elixir_vault_software PROPERTIES LINK_FLAGS "-dynamiclib -undefined dynamic_lookup")
endif()

if(UNIX AND NOT APPLE)
set_target_properties(ockam_elixir_vault_software PROPERTIES LINK_FLAGS "-fPIC -shared")
endif()

target_link_libraries(
  ockam_elixir_vault_software
  PUBLIC
    ockam::vault_interface
    ockam::vault_ffi
    ockam::vault_file
    ockam::vault_software
)
