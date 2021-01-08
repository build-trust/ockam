// Created by Ockam Developers

#ifndef RUST_KEY_EXCHANGE_H
#define RUST_KEY_EXCHANGE_H

#ifdef __cplusplus
extern "C" {
#endif

#include "vault.h"

typedef struct {
    int64_t handle;
    uint8_t kex_type;
} ockam_kex_t;

typedef struct {
    uint8_t h[32];
    uint64_t encrypt_key;
    uint64_t decrypt_key;
    uint8_t remote_static_public_key[65];
    size_t remote_static_public_key_len;
} ockam_completed_key_exchange_t;

/**
 * @brief  Create xx initiator.
 * @param   kex[out]         Resulting kex.
 * @param   vault[in]        Vault to use.
 * @param   identity_key[in] Identity key.
 * @return  error.
 */
ockam_vault_extern_error_t ockam_kex_xx_initiator(ockam_kex_t*         kex,
                                                  ockam_vault_t        vault,
                                                  ockam_vault_secret_t identity_key);

/**
 * @brief  Create xx responder.
 * @param   kex[out]         Resulting kex.
 * @param   vault[in]        Vault to use.
 * @param   identity_key[in] Identity key.
 * @return  error.
 */
ockam_vault_extern_error_t ockam_kex_xx_responder(ockam_kex_t*         kex,
                                                  ockam_vault_t        vault,
                                                  ockam_vault_secret_t identity_key);

/**
 * @brief  Process new portion of data.
 * @param   kex[in]              Kex object to use.
 * @param   data[in]             Data input.
 * @param   data_length[in]      Data length.
 * @param   response[out]        Response buffer.
 * @param   response_size[in]    Response buffer size.
 * @param   response_length[out] Response length.
 * @return  error.
 */
ockam_vault_extern_error_t ockam_kex_process(ockam_kex_t    kex,
                                             const uint8_t* data,
                                             size_t         data_length,
                                             uint8_t*       response,
                                             size_t         response_size,
                                             size_t*        response_length);

/**
 * @brief  Return whether kex is complete.
 * @param   kex[in]          Kex object to use.
 * @param   is_complete[out] Is complete.
 * @return  error.
 */
ockam_vault_extern_error_t ockam_kex_is_complete(ockam_kex_t kex,
                                                 bool*       is_complete);

/**
 * @brief  Return secret given its persistence id.
 * @param   kex[in]                 Kex object to use.
 * @param   completed_exchange[out] Resulting encryption data.
 * @return  error.
 */
ockam_vault_extern_error_t ockam_kex_finalize(ockam_kex_t                     kex,
                                              ockam_completed_key_exchange_t* completed_exchange);

#ifdef __cplusplus
} // extern "C"
#endif

#endif //RUST_KEY_EXCHANGE_H
