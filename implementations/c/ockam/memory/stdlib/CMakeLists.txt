
# ---
# ockam::memory_stdlib
# ---
add_library(ockam_memory_stdlib)
add_library(ockam::memory_stdlib ALIAS ockam_memory_stdlib)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)
target_include_directories(ockam_memory_stdlib PUBLIC ${INCLUDE_DIR})

file(COPY stdlib.h DESTINATION ${INCLUDE_DIR}/ockam/memory/)
target_sources(
  ockam_memory_stdlib
  PRIVATE
    stdlib.c
  PUBLIC
    ${INCLUDE_DIR}/ockam/memory/stdlib.h
)

target_link_libraries(ockam_memory_stdlib PUBLIC ockam::memory)

add_subdirectory(tests)
