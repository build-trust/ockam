
if(NOT BUILD_TESTING)
  return()
endif()

find_package(cmocka QUIET)
if(NOT cmocka_FOUND)
  return()
endif()

# ---
# ockam_queue_test
# ---
add_executable(ockam_queue_test queue_test.c)

target_link_libraries(
  ockam_queue_test
  PRIVATE
    cmocka-static
    ockam::log
    ockam::memory_stdlib
    ockam::queue
  )

add_test(ockam_queue_test ockam_queue_test)
