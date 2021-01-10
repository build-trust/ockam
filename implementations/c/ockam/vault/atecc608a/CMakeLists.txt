
# ---
# ockam::vault_atecc608a
# ---
add_library(ockam_vault_atecc608a)
add_library(ockam::vault_atecc608a ALIAS ockam_vault_atecc608a)

set(INCLUDE_DIR ${CMAKE_CURRENT_BINARY_DIR}/include)
target_include_directories(ockam_vault_atecc608a PUBLIC ${INCLUDE_DIR})

file(COPY atecc608a.h DESTINATION ${INCLUDE_DIR}/ockam/vault/)
target_sources(ockam_vault_atecc608a
    PRIVATE
        atecc608a.c
        ${INCLUDE_DIR}/ockam/vault/atecc608a.h
)

target_include_directories(ockam_vault_atecc608a
    PUBLIC
        "${cryptoauthlib_SOURCE_DIR}/lib"
        "${cryptoauthlib_SOURCE_DIR}/lib/hal"
        "${cryptoauthlib_BINARY_DIR}/lib")

target_link_directories(ockam_vault_atecc608a
        PUBLIC
        "${cryptoauthlib_BINARY_DIR}/lib")

target_link_libraries(ockam_vault_atecc608a
    PUBLIC
        ockam::mutex
        ockam::memory
        ockam::vault_interface
        ockam::vault
        ockam::log
        cryptoauth)

if (OCKAM_ENABLE_ATECC608A_TESTS)
    add_subdirectory(tests)
endif()
