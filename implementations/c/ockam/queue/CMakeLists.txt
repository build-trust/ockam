
# ---
# ockam::queue_pthread
# ---
add_library(ockam_queue)
add_library(ockam::queue ALIAS ockam_queue)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)

target_include_directories(ockam_queue PUBLIC ${INCLUDE_DIR})

file(COPY queue.h DESTINATION ${INCLUDE_DIR}/ockam)

target_sources(
  ockam_queue
  PRIVATE
    queue.c
    queue.h
)

target_link_libraries(
  ockam_queue
  PUBLIC
    ockam::log
    ockam::error_interface
    ockam::memory_stdlib
)

add_subdirectory(tests)
