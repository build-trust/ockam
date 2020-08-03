
# ---
# ockam::io_interface
# ---
add_library(ockam_io_interface INTERFACE)
add_library(ockam::io_interface ALIAS ockam_io_interface)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)

target_include_directories(ockam_io_interface INTERFACE ${INCLUDE_DIR})

file(COPY io.h DESTINATION ${INCLUDE_DIR}/ockam)
file(COPY impl.h DESTINATION ${INCLUDE_DIR}/ockam/io)

target_sources(
  ockam_io_interface
  INTERFACE
    ${INCLUDE_DIR}/ockam/io.h
    ${INCLUDE_DIR}/ockam/io/impl.h
)

# ---
# ockam::io
# ---
add_library(ockam_io)
add_library(ockam::io ALIAS ockam_io)

target_sources(
  ockam_io
  PRIVATE
    io.c
)

target_link_libraries(
  ockam_io
  PRIVATE
    ockam::log
  PUBLIC
    ockam::error_interface
    ockam::io_interface
)
