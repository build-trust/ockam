

# ---
# ockam::mutex_interface
# ---
add_library(ockam_mutex_interface INTERFACE)
add_library(ockam::mutex_interface ALIAS ockam_mutex_interface)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)

target_include_directories(ockam_mutex_interface INTERFACE ${INCLUDE_DIR})

file(COPY mutex.h DESTINATION ${INCLUDE_DIR}/ockam)
file(COPY impl.h DESTINATION ${INCLUDE_DIR}/ockam/mutex)

target_sources(
  ockam_mutex_interface
  INTERFACE
    ${INCLUDE_DIR}/ockam/mutex.h
    ${INCLUDE_DIR}/ockam/mutex/impl.h
)

# ---
# ockam::mutex
# ---
add_library(ockam_mutex)
add_library(ockam::mutex ALIAS ockam_mutex)

target_sources(
  ockam_mutex
  PRIVATE
    mutex.c
)

target_link_libraries(
  ockam_mutex
  PUBLIC
    ockam::error_interface
    ockam::mutex_interface
)
