add_subdirectory(posix_socket)

if(${OCKAM_BUILD_TESTS})
  add_subdirectory(tests)
  message(STATUS "-----TEST posix_tcp_transport--------")
endif()
