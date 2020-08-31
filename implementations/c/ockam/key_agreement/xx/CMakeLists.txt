
# ---
# ockam::key_agreement_xx
# ---
add_library(ockam_key_agreement_xx)
add_library(ockam::key_agreement_xx ALIAS ockam_key_agreement_xx)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)
target_include_directories(ockam_key_agreement_xx PUBLIC ${INCLUDE_DIR})

file(COPY xx.h DESTINATION ${INCLUDE_DIR}/ockam/key_agreement/)
file(COPY xx_local.h DESTINATION ${INCLUDE_DIR}/ockam/key_agreement/)
target_sources(
  ockam_key_agreement_xx
  PRIVATE
    xx.c
    xx_common.c
    xx_initiator.c
    xx_responder.c
    xx_local.h
  PUBLIC
    ${INCLUDE_DIR}/ockam/key_agreement/xx.h
)

target_link_libraries(
  ockam_key_agreement_xx
  PUBLIC
    ockam::error_interface
    ockam::log
    ockam::transport_interface
    ockam::io_interface
    ockam::memory_interface
    ockam::vault_default
    ockam::key_agreement
    ockam::codec
)

add_subdirectory(tests)
