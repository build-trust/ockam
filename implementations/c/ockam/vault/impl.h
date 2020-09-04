/**
 * @file    impl.h
 * @brief   Ockam Vault implementation interface
 */

#ifndef OCKAM_VAULT_IMPL_H_
#define OCKAM_VAULT_IMPL_H_

#include <ockam/error.h>
#include <ockam/vault.h>

#define OCKAM_VAULT_FEAT_RANDOM       0x01
#define OCKAM_VAULT_FEAT_SHA256       0x02
#define OCKAM_VAULT_FEAT_SECRET_ECDH  0x04
#define OCKAM_VAULT_FEAT_HKDF_SHA256  0x08
#define OCKAM_VAULT_FEAT_AEAD_AES_GCM 0x10
#define OCKAM_VAULT_FEAT_ALL          0x1F

typedef struct {
  /**
   * @brief   Deinitialize the specified ockam vault object
   * @param   vault[in] The ockam vault object to deinitialize.
   */
  ockam_error_t (*deinit)(ockam_vault_t* vault);

  /**
   * @brief   Generate a random number of desired size.
   * @param   vault[in]       Vault object to use for random number generation.
   * @param   buffer[out]     Buffer containing data to run through SHA-256.
   * @param   buffer_size[in] Size of the data to run through SHA-256.
   */
  ockam_error_t (*random)(ockam_vault_t* vault, uint8_t* buffer, size_t buffer_size);

  /**
   * @brief   Compute a SHA-256 hash based on input data.
   * @param   vault[in]           Vault object to use for SHA-256.
   * @param   input[in]           Buffer containing data to run through SHA-256.
   * @param   input_length[in]    Length of the data to run through SHA-256.
   * @param   digest[out]         Buffer to place the resulting SHA-256 hash in.
   * @param   digest_size[in]     Size of the digest buffer. Must be 32 bytes.
   * @param   digest_length[out]  Amount of data placed in the digest buffer.
   */
  ockam_error_t (*sha256)(ockam_vault_t* vault,
                          const uint8_t* input,
                          size_t         input_length,
                          uint8_t*       digest,
                          size_t         digest_size,
                          size_t*        digest_length);

  /**
   * @brief   Generate an ockam secret.
   * @param   vault[in]       Vault object to use for generating a secret key.
   * @param   secret[out]     Pointer to an ockam secret object to be populated with the generated secret.
   * @param   attributes[in]  Desired attribtes for the secret to be generated.
   */
  ockam_error_t (*secret_generate)(ockam_vault_t*                         vault,
                                   ockam_vault_secret_t*                  secret,
                                   const ockam_vault_secret_attributes_t* attributes);
  /**
   * @brief   Import the specified data into the supplied ockam vault secret.
   * @param   vault[in]         Vault object to use for generating a secret key.
   * @param   secret[out]       Pointer to an ockam secret object to be populated with input data.
   * @param   attributes[in]    Desired attribtes for the secret to be imported.
   * @param   input[in]         Data to load into the supplied secret.
   * @param   input_length[in]  Length of data to load into the secret.
   */
  ockam_error_t (*secret_import)(ockam_vault_t*                         vault,
                                 ockam_vault_secret_t*                  secret,
                                 const ockam_vault_secret_attributes_t* attributes,
                                 const uint8_t*                         input,
                                 size_t                                 input_length);

  /**
   * @brief   Export data from an ockam vault secret into the supplied output buffer.
   * @param   vault[in]                 Vault object to use for exporting secret data.
   * @param   secret[in]                Ockam vault secret to export data from.
   * @param   output_buffer[out]        Buffer to place the exported secret data in.
   * @param   output_buffer_size[in]    Size of the output buffer.
   * @param   output_buffer_length[out] Amount of data placed in the output buffer.
   * @return  OCKAM_ERROR_NONE on success.
   */
  ockam_error_t (*secret_export)(ockam_vault_t*        vault,
                                 ockam_vault_secret_t* secret,
                                 uint8_t*              output_buffer,
                                 size_t                output_buffer_size,
                                 size_t*               output_buffer_length);

  /**
   * @brief   Retrieve the public key from an ockam vault secret.
   * @param   vault[in]                 Vault object to use for exporting the public key
   * @param   secret[in]                Ockam vault secret to export the public key for.
   * @param   output_buffer[out]        Buffer to place the public key in.
   * @param   output_buffer_size[in]    Size of the output buffer.
   * @param   output_buffer_length[out] Amount of data placed in the output buffer.
   * @return  OCKAM_ERROR_NONE on success.
   */
  ockam_error_t (*secret_publickey_get)(ockam_vault_t*        vault,
                                        ockam_vault_secret_t* secret,
                                        uint8_t*              output_buffer,
                                        size_t                output_buffer_size,
                                        size_t*               output_buffer_length);

  /**
   * @brief   Retrive the attributes for a specified secret
   * @param   vault[in]       Vault object to use for retrieving ockam vault secret attributes.
   * @param   secret[in]      Ockam vault secret to get attributes for.
   * @param   attributes[out] Pointer to the attributes for the specified secret.
   */
  ockam_error_t (*secret_attributes_get)(ockam_vault_t*                   vault,
                                         ockam_vault_secret_t*            secret,
                                         ockam_vault_secret_attributes_t* attributes);

  /**
   * @brief   Set or change the type of secret. Note: EC secrets can not be changed.
   * @param   vault[in]   Vault object to use for setting secret type.
   * @param   secret[in]  Secret to change the type.
   * @param   type[in]    Type of secret to change to.
   */
  ockam_error_t (*secret_type_set)(ockam_vault_t* vault, ockam_vault_secret_t* secret, ockam_vault_secret_type_t type);

  /**
   * @brief   Delete an ockam vault secret.
   * @param   vault[in]   Vault object to use for deleting the ockam vault secret.
   * @param   secret[in]  Ockam vault secret to delete.
   * @return  OCKAM_ERROR_NONE on success.
   */
  ockam_error_t (*secret_destroy)(ockam_vault_t* vault, ockam_vault_secret_t* secret);

  /**
   * @brief   Perform an ECDH operation on the supplied ockam vault secret and peer_publickey. The result is another
   *          ockam vault secret of type unknown.
   * @param   vault[in]                 Vault object to use for encryption.
   * @param   privatekey[in]            The ockam vault secret to use for the private key of ECDH.
   * @param   peer_publickey[in]        Public key data to use for ECDH.
   * @param   peer_publickey_length[in] Length of the public key.
   * @param   shared_secret[out]        Resulting shared secret from a sucessful ECDH operation. Invalid if ECDH failed.
   * @return  OCKAM_ERROR_NONE on success.
   */
  ockam_error_t (*ecdh)(ockam_vault_t*        vault,
                        ockam_vault_secret_t* privatekey,
                        const uint8_t*        peer_publickey,
                        size_t                peer_publickey_length,
                        ockam_vault_secret_t* shared_secret);

  /**
   * @brief   Perform an HMAC-SHA256 based key derivation function on the supplied salt and input key material.
   * @param   vault[in]                 Vault object to use for encryption.
   * @param   salt[in]                  Ockam vault secret containing the salt for HKDF.
   * @param   input_key_material[in]    Ockam vault secret containing input key material to use for HKDF.
   * @param   derived_outputs_count[in] Total number of keys to generate.
   * @param   derived_outputs[out]      Array of ockam vault secrets resulting from HKDF.
   * @return  OCKAM_ERROR_NONE on success.
   */
  ockam_error_t (*hkdf_sha256)(ockam_vault_t*        vault,
                               ockam_vault_secret_t* salt,
                               ockam_vault_secret_t* input_key_material,
                               uint8_t               derived_outputs_count,
                               ockam_vault_secret_t* derived_outputs);

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
   * @return  OCKAM_ERROR_NONE on success.
   */
  ockam_error_t (*aead_aes_gcm_encrypt)(ockam_vault_t*        vault,
                                        ockam_vault_secret_t* key,
                                        uint16_t              nonce,
                                        const uint8_t*        additional_data,
                                        size_t                additional_data_length,
                                        const uint8_t*        plaintext,
                                        size_t                plaintext_length,
                                        uint8_t*              ciphertext_and_tag,
                                        size_t                ciphertext_and_tag_size,
                                        size_t*               ciphertext_and_tag_length);

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
   * @return  OCKAM_ERROR_NONE on success.
   */
  ockam_error_t (*aead_aes_gcm_decrypt)(ockam_vault_t*        vault,
                                        ockam_vault_secret_t* key,
                                        uint16_t              nonce,
                                        const uint8_t*        additional_data,
                                        size_t                additional_data_length,
                                        const uint8_t*        ciphertext_and_tag,
                                        size_t                ciphertext_and_tag_length,
                                        uint8_t*              plaintext,
                                        size_t                plaintext_size,
                                        size_t*               plaintext_length);
} ockam_vault_dispatch_table_t;

/**
 * @struct  ockam_vault_t
 * @brief
 */
struct ockam_vault {
  ockam_vault_dispatch_table_t* dispatch;
  void*                         context;
};

#endif
