
if(NOT BUILD_TESTING)
  return()
endif()

find_package(cmocka QUIET)
if(NOT cmocka_FOUND)
  return()
endif()

# ---
# ockam_memory_compare_test
# ---
add_executable(ockam_memory_compare_test memory_compare_test.c)

target_link_libraries(
  ockam_memory_compare_test
  PRIVATE
    ockam::log
    ockam::memory_stdlib
    cmocka-static
)

add_test(ockam_memory_compare_test ockam_memory_compare_test)
