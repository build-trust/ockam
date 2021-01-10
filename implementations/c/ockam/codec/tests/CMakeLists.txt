
if(NOT BUILD_TESTING)
  return()
endif()

find_package(cmocka QUIET)
if(NOT cmocka_FOUND)
  return()
endif()

# ---
# codec_tests
# ---
add_executable(ockam_codec_tests
  codec.c
  variable_length_encoded_u2le_test.c
  public_key_test.c
  endpoint_local_test.c
  endpoint_channel_test.c
  endpoint_test.c
  codec_tests.h
  route_test.c
)

target_link_libraries(
  ockam_codec_tests
  PRIVATE
  cmocka-static
    ockam::log
    ockam::codec
)

add_test(ockam_codec_tests ockam_codec_tests)
