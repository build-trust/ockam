
# ---
# ockam::log_interface
# ---
add_library(ockam_log_interface INTERFACE)
add_library(ockam::log_interface ALIAS ockam_log_interface)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)

target_include_directories(ockam_log_interface INTERFACE ${INCLUDE_DIR})

file(COPY log.h DESTINATION ${INCLUDE_DIR}/ockam)

target_sources(
  ockam_log_interface
  INTERFACE
    ${INCLUDE_DIR}/ockam/log.h
)