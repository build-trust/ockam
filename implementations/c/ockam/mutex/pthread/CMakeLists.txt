
# ---
# ockam::mutex_pthread
# ---
add_library(ockam_mutex_pthread)
add_library(ockam::mutex_pthread ALIAS ockam_mutex_pthread)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)
target_include_directories(ockam_mutex_pthread PUBLIC ${INCLUDE_DIR})

file(COPY pthread.h DESTINATION ${INCLUDE_DIR}/ockam/mutex/)
target_sources(
  ockam_mutex_pthread
  PRIVATE
    pthread.c
  PUBLIC
    ${INCLUDE_DIR}/ockam/mutex/pthread.h
)

target_link_libraries(
  ockam_mutex_pthread
  PRIVATE
    ockam::memory_interface
  PUBLIC
    ockam::mutex
)
