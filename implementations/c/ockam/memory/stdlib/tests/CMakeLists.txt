
if(NOT BUILD_TESTING)
  return()
endif()

find_package(cmocka QUIET)
if(NOT cmocka_FOUND)
  return()
endif()

# ---
# memory_compare_test
# ---
add_executable(memory_compare_test memory_compare_test.c)

target_link_libraries(
  memory_compare_test
  PRIVATE
    ockam::log
    ockam::memory_stdlib
    cmocka-static
)

add_test(memory_compare_test memory_compare_test)
