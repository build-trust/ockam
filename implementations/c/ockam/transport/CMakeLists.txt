
# ---
# ockam::transport_interface
# ---
add_library(ockam_transport_interface INTERFACE)
add_library(ockam::transport_interface ALIAS ockam_transport_interface)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)

target_include_directories(ockam_transport_interface INTERFACE ${INCLUDE_DIR})

file(COPY transport.h DESTINATION ${INCLUDE_DIR}/ockam)
file(COPY impl.h DESTINATION ${INCLUDE_DIR}/ockam/transport)

target_sources(
  ockam_transport_interface
  INTERFACE
    ${INCLUDE_DIR}/ockam/transport.h
    ${INCLUDE_DIR}/ockam/transport/impl.h
)

# ---
# ockam::transport
# ---
add_library(ockam_transport)
add_library(ockam::transport ALIAS ockam_transport)

target_sources(
  ockam_transport
  PRIVATE
    transport.c
)

target_link_libraries(
  ockam_transport
  PUBLIC
    ockam::error_interface
    ockam::transport_interface
    ockam::io
    ockam::memory
  ockam::codec
)
