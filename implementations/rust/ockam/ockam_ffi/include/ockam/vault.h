// Created by Ockam Developers

#ifndef RUST_VAULT_H
#define RUST_VAULT_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    int64_t handle;
    uint8_t vault_type;
} ockam_vault_t;

typedef uint64_t ockam_vault_secret_t;

/**
 * @struct ockam_vault_extern_error_t
 * @brief Represents an error that occurred in one of the `ockam_vault` functions.
 *
 * In the case of an error, resources associated with this error (the `domain` string)
 * must be released using @ref ockam_vault_free_error (which is a no-op if an error did
 * not occur) in order to avoid a memory leak.
 */
typedef struct {
    int32_t code;
    const char *domain;
} ockam_vault_extern_error_t;

/**
 * @enum    ockam_vault_secret_t
 * @brief   Supported secret types for AES and Elliptic Curves.
 */
typedef enum {
    OCKAM_VAULT_SECRET_TYPE_BUFFER = 0,
    OCKAM_VAULT_SECRET_TYPE_AES_KEY,
    OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY,
    OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY,
} ockam_vault_secret_type_t;

/**
 * @enum    ockam_vault_secret_persistence_t
 * @brief   Types of secrets vault can handle.
 */
typedef enum {
    OCKAM_VAULT_SECRET_EPHEMERAL = 0,
    OCKAM_VAULT_SECRET_PERSISTENT = 1,
} ockam_vault_secret_persistence_t;

/**
 * @struct  ockam_vault_secret_attributes_t
 * @brief   Attributes for a specific ockam vault secret.
 */
typedef struct {
    uint8_t  type;
    uint8_t  persistence;
    uint32_t length;
} ockam_vault_secret_attributes_t;

/**
 * @brief   Initialize the specified ockam vault object
 * @param   vault[out] The ockam vault object to initialize with the default vault.
 * @return  an error, which should be freed using @ref ockam_vault_free_error.
 */
ockam_vault_extern_error_t ockam_vault_default_init(ockam_vault_t* vault);

/**
 * @brief   Compute a SHA-256 hash based on input data.
 * @param   vault[in]           Vault object to use for SHA-256.
 * @param   input[in]           Buffer containing data to run through SHA-256.
 * @param   input_length[in]    Length of the data to run through SHA-256.
 * @param   digest[out]         Buffer to place the resulting SHA-256 hash in. Must be 32 bytes.
 * @return  an error, which should be freed using @ref ockam_vault_free_error.
 */
ockam_vault_extern_error_t ockam_vault_sha256(ockam_vault_t  vault,
                                              const uint8_t* input,
                                              uint32_t       input_length,
                                              uint8_t*       digest);

/**
 * @brief   Generate an ockam secret. Attributes struct must specify the configuration for the type of secret to
 *          generate. For EC keys and AES keys, length is ignored.
 * @param   vault[in]       Vault object to use for generating a secret key.
 * @param   secret[out]     Pointer to an ockam secret object to be populated with a handle to the secret
 * @param   attributes[in]  Desired attributes for the secret to be generated.
 * @return  an error, which should be freed using @ref ockam_vault_free_error.
 */
ockam_vault_extern_error_t ockam_vault_secret_generate(ockam_vault_t                   vault,
                                                       ockam_vault_secret_t*           secret,
                                                       ockam_vault_secret_attributes_t attributes);

/**
 * @brief   Import the specified data into the supplied ockam vault secret.
 * @param   vault[in]         Vault object to use for generating a secret key.
 * @param   secret[out]       Pointer to an ockam secret object to be populated with input data.
 * @param   attributes[in]    Desired attributes for the secret being imported.
 * @param   input[in]         Data to load into the supplied secret.
 * @param   input_length[in]  Length of data to load into the secret.
 * @return  an error, which should be freed using @ref ockam_vault_free_error.
 */

ockam_vault_extern_error_t ockam_vault_secret_import(ockam_vault_t                   vault,
                                                     ockam_vault_secret_t*           secret,
                                                     ockam_vault_secret_attributes_t attributes,
                                                     const uint8_t*                  input,
                                                     uint32_t                        input_length);

/**
 * @brief   Export data from an ockam vault secret into the supplied output buffer.
 * @param   vault[in]                 Vault object to use for exporting secret data.
 * @param   secret[in]                Ockam vault secret to export data from.
 * @param   output_buffer[out]        Buffer to place the exported secret data in.
 * @param   output_buffer_size[in]    Size of the output buffer.
 * @param   output_buffer_length[out] Amount of data placed in the output buffer.
 * @return  an error, which should be freed using @ref ockam_vault_free_error.
 */
ockam_vault_extern_error_t ockam_vault_secret_export(ockam_vault_t        vault,
                                                     ockam_vault_secret_t secret,
                                                     uint8_t*             output_buffer,
                                                     uint32_t             output_buffer_size,
                                                     uint32_t*            output_buffer_length);

/**
 * @brief   Retrieve the public key from an ockam vault secret.
 * @param   vault[in]                 Vault object to use for exporting the public key
 * @param   secret[in]                Ockam vault secret to export the public key for.
 * @param   output_buffer[out]        Buffer to place the public key in.
 * @param   output_buffer_size[in]    Size of the output buffer.
 * @param   output_buffer_length[out] Amount of data placed in the output buffer.
 * @return  an error, which should be freed using @ref ockam_vault_free_error.
 */
ockam_vault_extern_error_t ockam_vault_secret_publickey_get(ockam_vault_t        vault,
                                                            ockam_vault_secret_t secret,
                                                            uint8_t*             output_buffer,
                                                            uint32_t             output_buffer_size,
                                                            uint32_t*            output_buffer_length);

/**
 * @brief   Retrieve the attributes for a specified secret
 * @param   vault[in]               Vault object to use for retrieving ockam vault secret attributes.
 * @param   secret[in]              Ockam vault secret to get attributes for.
 * @param   secret_attributes[out]  Pointer to the attributes for the specified secret.
 * @return  an error, which should be freed using @ref ockam_vault_free_error.
 */
ockam_vault_extern_error_t ockam_vault_secret_attributes_get(ockam_vault_t                    vault,
                                                             uint64_t                         secret,
                                                             ockam_vault_secret_attributes_t* attributes);

/**
 * @brief   Delete an ockam vault secret.
 * @param   vault[in]   Vault object to use for deleting the ockam vault secret.
 * @param   secret[in]  Ockam vault secret to delete.
 * @return  an error, which should be freed using @ref ockam_vault_free_error.
 */
ockam_vault_extern_error_t ockam_vault_secret_destroy(ockam_vault_t vault, ockam_vault_secret_t secret);

/**
* @brief   Perform an ECDH operation on the supplied ockam vault secret and peer_publickey. The result is another
*          ockam vault secret of type unknown.
* @param   vault[in]                 Vault object to use for encryption.
* @param   privatekey[in]            The ockam vault secret to use for the private key of ECDH.
* @param   peer_publickey[in]        Public key data to use for ECDH.
* @param   peer_publickey_length[in] Length of the public key.
* @param   shared_secret[out]        Resulting shared secret from a successful ECDH operation. Invalid if ECDH failed.
* @return  an error, which should be freed using @ref ockam_vault_free_error.
*/
ockam_vault_extern_error_t ockam_vault_ecdh(ockam_vault_t         vault,
                                            ockam_vault_secret_t  privatekey,
                                            const uint8_t*        peer_publickey,
                                            uint32_t              peer_publickey_length,
                                            ockam_vault_secret_t* shared_secret);

/**
 * @brief   Perform an HMAC-SHA256 based key derivation function on the supplied salt and input key material.
 * @param   vault[in]                      Vault object to use for encryption.
 * @param   salt[in]                       Ockam vault secret containing the salt for HKDF.
 * @param   input_key_material[in]         Ockam vault secret containing input key material to use for HKDF.
 * @param   derived_outputs_attributes[in] Attributes of output secrets.
 * @param   derived_outputs_count[in]      Length of outputs attributes array.
 * @param   derived_outputs[out]           Array of ockam vault secrets resulting from HKDF.
 * @return  an error, which should be freed using @ref ockam_vault_free_error.
 */
ockam_vault_extern_error_t ockam_vault_hkdf_sha256(ockam_vault_t                          vault,
                                                   ockam_vault_secret_t                   salt,
                                                   const ockam_vault_secret_t*            input_key_material,
                                                   const ockam_vault_secret_attributes_t* derived_outputs_attributes,
                                                   uint8_t                                derived_outputs_count,
                                                   ockam_vault_secret_t*                  derived_outputs);

/**
 * @brief   Encrypt a payload using AES-GCM.
 * @param   vault[in]                       Vault object to use for encryption.
 * @param   key[in]                         Ockam secret key to use for encryption.
 * @param   nonce[in]                       Nonce value to use for encryption.
 * @param   additional_data[in]             Additional data to use for encryption.
 * @param   additional_data_length[in]      Length of the additional data.
 * @param   plaintext[in]                   Buffer containing plaintext data to encrypt.
 * @param   plaintext_length[in]            Length of plaintext data to encrypt.
 * @param   ciphertext_and_tag[in]          Buffer containing the generated ciphertext and tag data.
 * @param   ciphertext_and_tag_size[in]     Size of the ciphertext + tag buffer. Must be plaintext_size + 16.
 * @param   ciphertext_and_tag_length[out]  Amount of data placed in the ciphertext + tag buffer.
 * @return  an error, which should be freed using @ref ockam_vault_free_error.
 */
ockam_vault_extern_error_t ockam_vault_aead_aes_gcm_encrypt(ockam_vault_t        vault,
                                                            ockam_vault_secret_t key,
                                                            uint64_t             nonce,
                                                            const uint8_t*       additional_data,
                                                            uint32_t             additional_data_length,
                                                            const uint8_t*       plaintext,
                                                            uint32_t             plaintext_length,
                                                            uint8_t*             ciphertext_and_tag,
                                                            uint32_t             ciphertext_and_tag_size,
                                                            uint32_t*            ciphertext_and_tag_length);

/**
 * @brief   Decrypt a payload using AES-GCM.
 * @param   vault[in]                     Vault object to use for decryption.
 * @param   key[in]                       Ockam secret key to use for decryption.
 * @param   nonce[in]                     Nonce value to use for decryption.
 * @param   additional_data[in]           Additional data to use for decryption.
 * @param   additional_data_length[in]    Length of the additional data.
 * @param   ciphertext_and_tag[in]        The ciphertext + tag data to decrypt.
 * @param   ciphertext_and_tag_length[in] Length of the ciphertext + tag data to decrypt.
 * @param   plaintext[out]                Buffer to place the decrypted data in.
 * @param   plaintext_size[in]            Size of the plaintext buffer. Must be ciphertext_tag_size - 16.
 * @param   plaintext_length[out]         Amount of data placed in the plaintext buffer.
 * @return  an error, which should be freed using @ref ockam_vault_free_error.
 */
ockam_vault_extern_error_t ockam_vault_aead_aes_gcm_decrypt(ockam_vault_t       vault,
                                                            ockam_vault_secret_t key,
                                                            uint64_t             nonce,
                                                            const uint8_t*       additional_data,
                                                            uint32_t             additional_data_length,
                                                            const uint8_t*       ciphertext_and_tag,
                                                            uint32_t             ciphertext_and_tag_length,
                                                            uint8_t*             plaintext,
                                                            uint32_t             plaintext_size,
                                                            uint32_t*            plaintext_length);

/**
 * @brief   Deinitialize the specified ockam vault object
 * @param   vault[in] The ockam vault object to deinitialize.
 * @return  an error, which should be freed using @ref ockam_vault_free_error.
 */
ockam_vault_extern_error_t ockam_vault_deinit(ockam_vault_t vault);

/**
 * @brief   Free any resources associated with a @ref ockam_vault_extern_error_t.
 * @param   error[in] the error to free.
 */
void ockam_vault_free_error(ockam_vault_extern_error_t *error);

#ifdef __cplusplus
} // extern "C"
#endif

#endif // RUST_VAULT_H
