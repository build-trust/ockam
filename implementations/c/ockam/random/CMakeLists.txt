
# ---
# ockam::random_interface
# ---
add_library(ockam_random_interface INTERFACE)
add_library(ockam::random_interface ALIAS ockam_random_interface)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)

target_include_directories(ockam_random_interface INTERFACE ${INCLUDE_DIR})

file(COPY random.h DESTINATION ${INCLUDE_DIR}/ockam)
file(COPY impl.h DESTINATION ${INCLUDE_DIR}/ockam/random)

target_sources(
  ockam_random_interface
  INTERFACE
    ${INCLUDE_DIR}/ockam/random.h
    ${INCLUDE_DIR}/ockam/random/impl.h
)

# ---
# ockam::random
# ---
add_library(ockam_random)
add_library(ockam::random ALIAS ockam_random)

target_sources(
  ockam_random
  PRIVATE
    random.c
)

target_link_libraries(
  ockam_random
  PUBLIC
    ockam::error_interface
    ockam::random_interface
)
