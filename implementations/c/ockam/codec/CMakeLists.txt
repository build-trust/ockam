
# ---
# ockam::codec
# ---
add_library(ockam_codec)
add_library(ockam::codec ALIAS ockam_codec)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)
target_include_directories(ockam_codec PUBLIC ${INCLUDE_DIR})

file(COPY codec.h DESTINATION ${INCLUDE_DIR}/ockam/)
target_sources(
  ockam_codec
  PRIVATE
    codec.c
    variable_length_encoded_u2le.c
    payload_aead_aes_gcm.c
    public_key.c
    payload.c
    endpoint.c
    key_agreement.c
    ockam_wire.c
    route.c
  PUBLIC
    ${INCLUDE_DIR}/ockam/codec.h
)

target_link_libraries(ockam_codec PUBLIC ockam::error_interface ockam::log)

add_subdirectory(tests)
