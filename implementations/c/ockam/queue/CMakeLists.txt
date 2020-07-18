
# ---
# ockam::queue_interface
# ---
add_library(ockam_queue_interface INTERFACE)
add_library(ockam::queue_interface ALIAS ockam_queue_interface)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)

target_include_directories(ockam_queue_interface INTERFACE ${INCLUDE_DIR})

file(COPY queue.h DESTINATION ${INCLUDE_DIR}/ockam)

target_sources(
  ockam_queue_interface
  INTERFACE
    ${INCLUDE_DIR}/ockam/queue.h
)
