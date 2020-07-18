
# ---
# ockam::error_interface
# ---
add_library(ockam_error_interface INTERFACE)
add_library(ockam::error_interface ALIAS ockam_error_interface)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)

file(COPY error.h DESTINATION ${INCLUDE_DIR}/ockam)

target_include_directories(ockam_error_interface INTERFACE ${INCLUDE_DIR})

target_sources(
  ockam_error_interface
  INTERFACE
    ${INCLUDE_DIR}/ockam/error.h
)
