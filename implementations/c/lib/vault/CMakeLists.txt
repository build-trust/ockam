add_subdirectory(default)

#add_subdirectory(atecc508a)
#add_subdirectory(atecc608a)

if(${OCKAM_BUILD_TESTS})
  add_subdirectory(tests)
endif()
