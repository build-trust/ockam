
if(VAULT_IFACE_I2C)
    set(ATCA_HAL_I2C ON CACHE BOOL "")
endif()
add_subdirectory(${THIRD_PARTY_DIR}/microchip/cryptoauthlib cryptoauthlib)
