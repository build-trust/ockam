add_subdirectory(default)
add_subdirectory(atecc508a)

if(${OCKAM_BUILD_TESTS})
  add_subdirectory(tests)
endif()
