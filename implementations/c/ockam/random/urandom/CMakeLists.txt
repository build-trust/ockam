
# ---
# ockam::random_urandom
# ---
add_library(ockam_random_urandom)
add_library(ockam::random_urandom ALIAS ockam_random_urandom)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)
target_include_directories(ockam_random_urandom PUBLIC ${INCLUDE_DIR})

file(COPY urandom.h DESTINATION ${INCLUDE_DIR}/ockam/random/)
target_sources(
  ockam_random_urandom
  PRIVATE
    urandom.c
  PUBLIC
    ${INCLUDE_DIR}/ockam/random/urandom.h
)

target_link_libraries(ockam_random_urandom PUBLIC ockam::random)
