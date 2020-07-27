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
 * @enum    ockam_vault_secret_t
 * @brief   Supported secret types for AES and Elliptic Curves.
 */
typedef enum {
    OCKAM_VAULT_SECRET_TYPE_BUFFER = 0,
    OCKAM_VAULT_SECRET_TYPE_AES128_KEY,
    OCKAM_VAULT_SECRET_TYPE_AES256_KEY,
    OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY,
    OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY,
} ockam_vault_secret_type_t;

/**
 * @enum    ockam_vault_secret_persistence_t
 * @brief   Types of secrets vault can handle.
 */
typedef enum {
    OCKAM_VAULT_SECRET_EPHEMERAL = 0,
    OCKAM_VAULT_SECRET_PERSISTENT,
} ockam_vault_secret_persistence_t;

/**
 * @enum    ockam_vault_secret_purpose_t
 * @brief   Types of uses for a secret
 */
typedef enum {
    OCKAM_VAULT_SECRET_PURPOSE_KEY_AGREEMENT = 0,
} ockam_vault_secret_purpose_t;

/**
 * @struct  ockam_vault_secret_attributes_t
 * @brief   Attributes for a specific ockam vault secret.
 */
typedef struct {
    uint16_t                         length;
    ockam_vault_secret_type_t        type;
    ockam_vault_secret_purpose_t     purpose;
    ockam_vault_secret_persistence_t persistence;
} ockam_vault_secret_attributes_t;


/**
 * @struct  ockam_vault_secret_attributes_t
 * @brief   Attributes for a specific ockam vault secret.
 */
typedef struct {
    ockam_vault_secret_type_t        type;
    ockam_vault_secret_purpose_t     purpose;
    ockam_vault_secret_persistence_t persistence;
} ockam_vault_secret_attributes_t;


typedef struct {
    ockam_vault_secret_attributes_t attributes;
    char* handle;
} ockam_vault_secret_t;

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
 * @brief   Compute a SHA-256 hash based on input data.
 * @param   vault[in]           Vault object to use for SHA-256.
 * @param   input[in]           Buffer containing data to run through SHA-256.
 * @param   input_length[in]    Length of the data to run through SHA-256.
 * @param   digest[out]         Buffer to place the resulting SHA-256 hash in. Must be 32 bytes.
 * @return  OCKAM_ERROR_NONE on success.
 */
uint32_t ockam_vault_sha256(ockam_vault_t vault,
                            const uint8_t* const input,
                            size_t input_length,
                            uint8_t* digest);

/**
 * @brief   Generate an ockam secret. Attributes struct must specify the configuration for the type of secret to
 *          generate. For EC keys and AES keys, length is ignored.
 * @param   vault[in]       Vault object to use for generating a secret key.
 * @param   secret[out]     Pointer to an ockam secret object to be populated with a handle to the secret
 * @param   attributes[in]  Desired attribtes for the secret to be generated.
 */
uint32_t ockam_vault_secret_generate(ockam_vault_t                   vault,
                                     ockam_vault_secret_t*           secret,
                                     ockam_vault_secret_attributes_t attributes);

/**
 * @brief   Import the specified data into the supplied ockam vault secret.
 * @param   vault[in]         Vault object to use for generating a secret key.
 * @param   secret[out]       Pointer to an ockam secret object to be populated with input data.
 * @param   attributes[in]    Desired attributes for the secret being imported.
 * @param   input[in]         Data to load into the supplied secret.
 * @param   input_length[in]  Length of data to load into the secret.
 */

uint32_t ockam_vault_secret_import(ockam_vault_t                   vault,
                                   ockam_vault_secret_t*           secret,
                                   ockam_vault_secret_attributes_t attributes,
                                   const uint8_t* const            input,
                                   size_t                          input_length);

/**
 * @brief   Export data from an ockam vault secret into the supplied output buffer.
 * @param   vault[in]                 Vault object to use for exporting secret data.
 * @param   secret[in]                Ockam vault secret to export data from.
 * @param   output_buffer[out]        Buffer to place the exported secret data in.
 * @param   output_buffer_size[in]    Size of the output buffer.
 * @param   output_buffer_length[out] Amount of data placed in the output buffer.
 * @return  OCKAM_ERROR_NONE on success.
 */
uint32_t ockam_vault_secret_export(ockam_vault_t        vault,
                                   ockam_vault_secret_t secret,
                                   uint8_t*             output_buffer,
                                   size_t               output_buffer_size,
                                   size_t*              output_buffer_length);

/**
 * @brief   Retrieve the public key from an ockam vault secret.
 * @param   vault[in]                 Vault object to use for exporting the public key
 * @param   secret[in]                Ockam vault secret to export the public key for.
 * @param   output_buffer[out]        Buffer to place the public key in.
 * @param   output_buffer_size[in]    Size of the output buffer.
 * @param   output_buffer_length[out] Amount of data placed in the output buffer.
 * @return  OCKAM_ERROR_NONE on success.
 */
uint32_t ockam_vault_secret_publickey_get(ockam_vault_t        vault,
                                          ockam_vault_secret_t secret,
                                          uint8_t*             output_buffer,
                                          size_t               output_buffer_size,
                                          size_t*              output_buffer_length);

/**
 * @brief   Retrieve the attributes for a specified secret
 * @param   vault[in]               Vault object to use for retrieving ockam vault secret attributes.
 * @param   secret[in]              Ockam vault secret to get attributes for.
 * @param   secret_attributes[out]  Pointer to the attributes for the specified secret.
 */
ockam_error_t ockam_vault_secret_attributes_get(ockam_vault_t                    vault,
                                                ockam_vault_secret_t             secret,
                                                ockam_vault_secret_attributes_t* attributes);

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