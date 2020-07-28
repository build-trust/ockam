# ---
# ockam::log
# ---
add_library(ockam_log)
add_library(ockam::log ALIAS ockam_log)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)

target_include_directories(ockam_log_interface INTERFACE ${INCLUDE_DIR})

file(COPY syslog.h DESTINATION ${INCLUDE_DIR}/ockam/log)

target_sources(
  ockam_log
  PRIVATE
    ${INCLUDE_DIR}/ockam/log/syslog.h
    syslog.c
)

target_link_libraries(
  ockam_log
  PUBLIC
    ockam::error_interface
    ockam::log_interface
)
