
# ---
# ockam::queue_pthread
# ---
add_library(ockam_queue_pthread)
add_library(ockam::queue_pthread ALIAS ockam_queue_pthread)

target_sources(
  ockam_queue_pthread
  PRIVATE
    queue.c
)

target_link_libraries(
  ockam_queue_pthread
  PUBLIC
    ockam::log
    ockam::error_interface
    ockam::memory_stdlib
    ockam::queue_interface
)

add_subdirectory(tests)
