/**
 * @file    vault.h
 * @brief   Vault interface for the Ockam Library
 */

#ifndef OCKAM_VAULT_H_
#define OCKAM_VAULT_H_

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @struct ockam_vault_t
 * @brief An ockam vault instance
 */
typedef struct {
    int64_t handle;
    int32_t vault_id;
} ockam_vault_t;

/**
 * @brief   Initialize the specified ockam vault object
 * @param   vault[out] The ockam vault object to initialize with the default vault.
 * @return  OCKAM_ERROR_NONE on success.
 */
uint32_t ockam_vault_default_init(ockam_vault_t* vault);

/**
 * @brief   Generate a random number of desired size.
 * @param   vault[in]       Vault object to use for random number generation.
 * @param   buffer[out]     Buffer that is filled
 * @param   buffer_size[in] Size of the data
 * @return  OCKAM_ERROR_NONE on success.
 */
uint32_t ockam_vault_random_bytes_generate(ockam_vault_t vault, uint8_t* buffer, size_t buffer_size);

/**
 * @brief   Deinitialize the specified ockam vault object
 * @param   vault[in] The ockam vault object to deinitialize.
 * @return  OCKAM_ERROR_NONE on success.
 */
uint32_t ockam_vault_deinit(ockam_vault_t vault);


#ifdef __cplusplus
} // extern "C"
#endif

#endif