set(VAULT_INC ${VAULT_INC} ${VAULT_EXTERNAL_SRC_DIR}/microchip/)
set(VAULT_INC ${VAULT_INC} ${THIRD_PARTY_DIR}/microchip/)
set(VAULT_INC ${VAULT_INC} ${THIRD_PARTY_DIR}/microchip/cryptoauthlib/lib)
set(VAULT_INC ${VAULT_INC} ${THIRD_PARTY_DIR}/microchip/cryptoauthlib/lib/hal)

if(VAULT_TPM_IFACE_I2C)
    set(ATCA_HAL_I2C ON CACHE BOOL "")
    set(ATCA_BUILD_SHARED_LIBS OFF CACHE BOOL "")
endif()

if(VAULT_TPM_IFACE_SPI)
    set(ATCA_HAL_SPI ON CACHE BOOL "")
    set(ATCA_BUILD_SHARED_LIBS OFF CACHE BOOL "")
endif()

if(VAULT_TPM_IFACE_SPI)
    set(ATCA_HAL_SPI ON CACHE BOOL "")
    set(ATCA_BUILD_SHARED_LIBS OFF CACHE BOOL "")
endif()

add_subdirectory(${THIRD_PARTY_DIR}/microchip/cryptoauthlib cryptoauthlib)
