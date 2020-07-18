
# ---
# ockam::key_agreement_interface
# ---
add_library(ockam_key_agreement_interface INTERFACE)
add_library(ockam::key_agreement_interface ALIAS ockam_key_agreement_interface)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)

target_include_directories(ockam_key_agreement_interface INTERFACE ${INCLUDE_DIR})

file(COPY key_agreement.h DESTINATION ${INCLUDE_DIR}/ockam)
file(COPY impl.h DESTINATION ${INCLUDE_DIR}/ockam/key_agreement)

target_sources(
  ockam_key_agreement_interface
  INTERFACE
    ${INCLUDE_DIR}/ockam/key_agreement.h
    ${INCLUDE_DIR}/ockam/key_agreement/impl.h
)

# ---
# ockam::key_agreement
# ---
add_library(ockam_key_agreement)
add_library(ockam::key_agreement ALIAS ockam_key_agreement)

target_sources(
  ockam_key_agreement
  PRIVATE
    key_impl.c
)

target_link_libraries(
  ockam_key_agreement
  PRIVATE
    ockam::log
    ockam::memory_interface
  PUBLIC
    ockam::error_interface
    ockam::key_agreement_interface
)
