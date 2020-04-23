/**
 * @file    impl.h
 * @brief   Ockam Vault implementation interface
 */

#ifndef OCKAM_VAULT_IMPL_H_
#define OCKAM_VAULT_IMPL_H_

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
   * @param   vault[in]             Vault object to use for generating a secret key.
   * @param   secret[out]           Pointer to an ockam secret object to be populated with the generated secret.
   * @param   secret_attributes[in] Desired attribtes for the secret to be generated.
   */
  ockam_error_t (*secret_generate_random)(ockam_vault_t*                   vault,
                                          ockam_vault_secret_t*            secret,
                                          ockam_vault_secret_attributes_t* secret_attributes);

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
  ockam_error_t (*aead_aes_128_gcm_encrypt)(ockam_vault_t*       vault,
                                            ockam_vault_secret_t key,
                                            uint16_t             nonce,
                                            const uint8_t*       additional_data,
                                            size_t               additional_data_length,
                                            const uint8_t*       plaintext,
                                            size_t               plaintext_length,
                                            uint8_t*             ciphertext_and_tag,
                                            size_t               ciphertext_and_tag_size,
                                            size_t*              ciphertext_and_tag_length);

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
  ockam_error_t (*aead_aes_128_gcm_decrypt)(ockam_vault_t*       vault,
                                            ockam_vault_secret_t key,
                                            uint16_t             nonce,
                                            const uint8_t*       additional_data,
                                            size_t               additional_data_length,
                                            const uint8_t*       ciphertext_and_tag,
                                            size_t               ciphertext_and_tag_length,
                                            uint8_t*             plaintext,
                                            size_t               plaintext_size,
                                            size_t*              plaintext_length);
} ockam_vault_dispatch_table_t;

/**
 * @struct  ockam_vault_t
 * @brief
 */
struct ockam_vault_t {
  ockam_vault_dispatch_table_t* dispatch;
  void*                         context;
};

#endif
