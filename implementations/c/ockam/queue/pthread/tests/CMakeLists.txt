
if(NOT BUILD_TESTING)
  return()
endif()

find_package(cmocka QUIET)
if(NOT cmocka_FOUND)
  return()
endif()

# ---
# queue_test
# ---
add_executable(queue_test queue_test.c)

target_link_libraries(
  queue_test
  PRIVATE
    cmocka
    ockam::log
    ockam::memory_stdlib
    ockam::queue_pthread
  )

add_test(queue_test queue_test)
