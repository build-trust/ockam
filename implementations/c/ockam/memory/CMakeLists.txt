
# ---
# ockam::memory_interface
# ---
add_library(ockam_memory_interface INTERFACE)
add_library(ockam::memory_interface ALIAS ockam_memory_interface)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)

target_include_directories(ockam_memory_interface INTERFACE ${INCLUDE_DIR})

file(COPY memory.h DESTINATION ${INCLUDE_DIR}/ockam)
file(COPY impl.h DESTINATION ${INCLUDE_DIR}/ockam/memory)

target_sources(
  ockam_memory_interface
  INTERFACE
    ${INCLUDE_DIR}/ockam/memory.h
    ${INCLUDE_DIR}/ockam/memory/impl.h
)

# ---
# ockam::memory
# ---
add_library(ockam_memory)
add_library(ockam::memory ALIAS ockam_memory)

target_sources(
  ockam_memory
  PRIVATE
    memory.c
)

target_link_libraries(
  ockam_memory
  PUBLIC
    ockam::error_interface
    ockam::memory_interface
)
