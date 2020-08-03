
# ---
# ockam::channel_interface
# ---
add_library(ockam_channel_interface INTERFACE)
add_library(ockam::channel_interface ALIAS ockam_channel_interface)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)

target_include_directories(ockam_channel_interface INTERFACE ${INCLUDE_DIR})

file(COPY channel.h DESTINATION ${INCLUDE_DIR}/ockam)
file(COPY channel_impl.h DESTINATION ${INCLUDE_DIR}/ockam/channel)

target_sources(
  ockam_channel_interface
  INTERFACE
    ${INCLUDE_DIR}/ockam/channel.h
    ${INCLUDE_DIR}/ockam/channel/channel_impl.h
)

# ---
# ockam::channel
# ---
add_library(ockam_channel)
add_library(ockam::channel ALIAS ockam_channel)

target_sources(
  ockam_channel
  PRIVATE
    channel_impl.c
)

target_link_libraries(
  ockam_channel
  PRIVATE
    ockam::log
    ockam::memory_stdlib
    ockam::key_agreement_xx
  PUBLIC
    ockam::error_interface
    ockam::io_interface
    ockam::memory_interface
    ockam::vault_interface
    ockam::channel_interface
)

add_subdirectory(tests)
