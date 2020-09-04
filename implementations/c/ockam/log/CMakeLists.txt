# ---
# ockam::log
# ---
add_library(ockam_log)
add_library(ockam::log ALIAS ockam_log)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)

target_include_directories(ockam_log PUBLIC ${INCLUDE_DIR})

file(COPY log.h DESTINATION ${INCLUDE_DIR}/ockam)

target_sources(
  ockam_log
  PRIVATE
    ${INCLUDE_DIR}/ockam/log.h
    log.c
)

target_link_libraries(
  ockam_log
)

if (OCKAM_DISABLE_LOG)
  message(STATUS "Log is disabled")
  target_compile_definitions(ockam_log PUBLIC OCKAM_DISABLE_LOG)
else()
  message(STATUS "Log is enabled")
endif()

if (OCKAM_CUSTOM_LOG_FUNCTION)
  message(STATUS "Custom logging is available")
  target_compile_definitions(ockam_log PUBLIC OCKAM_CUSTOM_LOG_FUNCTION)
else()
  message(STATUS "Custom logging is disabled")
endif()