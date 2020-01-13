set(VAULT_INC ${VAULT_INC} ${THIRD_PARTY_DIR}/arm/mbed-crypto/include)

set(ENABLE_TESTING OFF CACHE BOOL "")
set(ENABLE_PROGRAMS OFF CACHE BOOL "")
add_subdirectory(${THIRD_PARTY_DIR}/arm/mbed-crypto mbed-crypto)
