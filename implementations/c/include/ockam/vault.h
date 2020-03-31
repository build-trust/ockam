/**
 ********************************************************************************************************
 * @file    vault.h
 * @brief   Vault interface for the Ockam Library
 ********************************************************************************************************
 */

#ifndef OCKAM_VAULT_H_
#define OCKAM_VAULT_H_

/*
 ********************************************************************************************************
 * @defgroup    OCKAM_VAULT OCKAM_VAULT_API
 * @ingroup     OCKAM
 * @brief       OCKAM_VAULT_API
 *
 * @addtogroup  OCKAM_VAULT
 * @{
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include <stddef.h>
#include <stdint.h>

#include "ockam/error.h"
#include "ockam/memory.h"

/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

#define OCKAM_VAULT_RANDOM 0x01   /*!< Features bit for Random                          */
#define OCKAM_VAULT_SHA256 0x02   /*!< Features bit for SHA256                          */
#define OCKAM_VAULT_KEY_ECDH 0x04 /*!< Features bit for Key/Ecdh                        */
#define OCKAM_VAULT_HKDF 0x08     /*!< Features bit for HKDF                            */
#define OCKAM_VAULT_AES_GCM 0x10  /*!< Features bit for AES-GCM                         */
#define OCKAM_VAULT_ALL 0x1F      /*!< Features bits to enable all Vault functions      */

#define kVaultErrorInvalidParam (kOckamErrorInterfaceVault | 0x01)
#define kVaultErrorMemoryFail (kOckamErrorInterfaceVault | 0x02)

/*
 ********************************************************************************************************
 *                                               CONSTANTS                                              *
 ********************************************************************************************************
 */

typedef OckamError VaultError;

/**
 *******************************************************************************
 * @enum    OckamVaultKey
 *
 * @brief   Supported key types in an Ockam Vault implementation
 *******************************************************************************
 */

typedef enum {
  kOckamVaultKeyStatic = 0, /*!< Static key                                       */
  kOckamVaultKeyEphemeral,  /*!< Ephemeral key                                    */
  kMaxOckamVaultKey         /*!< Total number of key types supported              */
} OckamVaultKey;

/**
 *******************************************************************************
 * @enum    OckamVaultEc
 *
 * @brief   Supported elliptic curve types in Ockam Vault. An Ockam Vault
 *          implementation must support at least one of these curves.
 *******************************************************************************
 */

typedef enum {
  kOckamVaultEcP256 = 0,   /*!< NIST P-256 Curve                                 */
  kOckamVaultEcCurve25519, /*!< Curve25519                                       */
  kMaxOckamVaultEc,        /*!< Total number of curves available                 */
  kOckamVaultEcNone        /*!< No EC specified or needed                        */
} OckamVaultEc;

/*
 ********************************************************************************************************
 *                                               DATA TYPES                                             *
 ********************************************************************************************************
 */

/**
 *******************************************************************************
 * @struct  OckamVaultCtx
 *
 * @brief   This struct is common for all Vault implementations. It contains all
 *          information about a single instance of Vault. Multiple instances of
 *          vault can be created by allocating multiple Vault context
 *          structures. The memory, features and ec parameters should always be
 *          set in this struct. The context pointers may or may not be used, it
 *          depends on the specific Vault implementation.
 *******************************************************************************
 */

typedef struct {
  const OckamMemory *memory; /*!< Pointer to the Ockam Memory interface            */
  uint32_t features;         /*!< Bitfield containing all enabled vault functions  */
  uint32_t default_features; /*!< Bitfield containing all enabled default functions*/
  OckamVaultEc ec;           /*!< The elliptic curve supported by this vault       */
  void *random_ctx;          /*!< Pointer to random function data                  */
  void *key_ecdh_ctx;        /*!< Pointer to the Key/ECDH function data            */
  void *sha256_ctx;          /*!< Pointer to the SHA256 function data              */
  void *hkdf_ctx;            /*!< Pointer to the HKDF function data                */
  void *aes_gcm_ctx;         /*!< Pointer to the AES-GCM function data             */
} OckamVaultCtx;

/*
 ********************************************************************************************************
 *                                               INTERFACE                                              *
 ********************************************************************************************************
 */

#ifdef __cplusplus
extern "C" {
#endif

/**
 *******************************************************************************
 * @struct  OckamVault
 *
 * @brief   This is the top-level interface for Vault. A specific Vault
 *          implementation will either define all functions listed below, or it
 *          will implemention a sub-set of these functions and link to the
 *          default vault for the rest of the functions.
 *******************************************************************************
 */

typedef struct {
  /**
   ****************************************************************************************************
   *                                          Create()
   *
   * @brief   Create an ockam instance of an Ockam Vault.
   *
   * @param   ctx[in,out] Pointer to a context structure to be initialized by Vault.
   *
   * @param   p_arg[in]   Configuration structure for the specific Vault implementation.
   *
   * @param   memory[in]  Pointer to an Ockam memory instance.
   *
   * @return  kOckamErrorNone on success.
   ****************************************************************************************************
   */

  OckamError (*Create)(void **ctx, void *p_arg, const OckamMemory *memory);

  /**
   ****************************************************************************************************
   *                                         Destroy()
   *
   * @brief   Destroy an instance of an Ockam Vault.
   *
   * @param   p_ctx[in] Pointer to a context structure to be destroyed.
   *
   * @return  kOckamErrorNone on success.
   ****************************************************************************************************
   */

  OckamError (*Destroy)(void *p_ctx);

  /**
   ****************************************************************************************************
   *                                         Random()
   *
   * @brief   Generate a random number of desired size.
   *
   * @param   p_ctx[in]     Pointer to an initialized Vault context structure.
   *
   * @param   p_num[out]    Pointer to a buffer to store the generated random number. Must be able to
   *                        fit the requested random number.
   *
   * @param   num_size[in]  Size of the random number to generate.
   *
   * @return  kOckamErrorNone on success.
   ****************************************************************************************************
   */

  OckamError (*Random)(void *p_ctx, uint8_t *p_num, size_t num_size);

  /**
   ****************************************************************************************************
   *                                         KeyGenerate()
   *
   * @brief   Generate a key-pair for the desired key type.
   *
   * @param   p_ctx[in]     Pointer to an initialized Vault context structure.
   *
   * @param   key_type[in]  The OckamVaultKey type to generate a public/private keypair for. If a
   *                        keypair already exists for the key-type, this will overwrite the existing
   *                        keypair.
   *
   * @return  kOckamErrorNone on success.
   ****************************************************************************************************
   */

  OckamError (*KeyGenerate)(void *p_ctx, OckamVaultKey key_type);

  /**
   ****************************************************************************************************
   *                                         KeyGetPublic()
   *
   * @brief   Retrive the public key for the specified key type.
   *
   * @param   p_ctx[in]         Pointer to an initialized Vault context structure.
   *
   * @param   key_type[in]      The OckamVaultKey to get the public key for.
   *
   * @param   p_pub_key[out]    Buffer to place the public key in.
   *
   * @param   pub_key_size[in]  Size of the public key buffer. Must match the size of the public key
   *                            that is being retrived.
   *
   * @return  kOckamErrorNone on success.
   ****************************************************************************************************
   */

  OckamError (*KeyGetPublic)(void *ctx, OckamVaultKey key_type, uint8_t *p_pub_key, size_t pub_key_size);

  /**
   ****************************************************************************************************
   *                                         KeySetPrivate()
   *
   * @brief   Write a private key to Vault. NOTE: This may not be supported on some Vault hardware
   *          instances. In general, the use of this function is discouraged outside of testing.
   *
   * @param   p_ctx[in]         Pointer to an initialized Vault context structure.
   *
   * @param   key_type[in]      The OckamVaultKey to write the private key to.
   *
   * @param   p_priv_key[in]    Buffer containing the private key to write to Vault.
   *
   * @param   priv_key_size[in] Size of the private key to write. It must match the size of the private
   *                            key for the specific elliptic curve in Vault.
   *
   * @return  kOckamErrorNone on success.
   ****************************************************************************************************
   */

  OckamError (*KeySetPrivate)(void *ctx, OckamVaultKey key_type, uint8_t *p_priv_key, size_t priv_key_size);

  /**
   ****************************************************************************************************
   *                                           ECDH()
   *
   * @brief   Compute a shared secret using Elliptic Curve Diffie-Hellman.
   *
   * @param   p_ctx[in]         Pointer to an initialized Vault context structure.
   *
   * @param   key_type[in]      The OckamVaultKey to use for the private key in ECDH.
   *
   * @param   p_pub_key[in]     Buffer containing the public key to use for ECDH.
   *
   * @param   pub_key_size[in]  Size of the public key. Must match the expected size for the supproted
   *                            elliptic curve in Vault.
   *
   * @param   p_ss[out]         Buffer to place the resuling shared secret in from ECDH. Contents of
   *                            the buffer are in a unknown state on an error condition.
   *
   * @param   ss_size[in]       Size of the shared secret buffer. Must match the expected shared
   *                            secret size.
   *
   * @return  kOckamErrorNone on success.
   ****************************************************************************************************
   */
  OckamError (*Ecdh)(void *ctx, OckamVaultKey key_type, uint8_t *p_pub_key, size_t pub_key_size, uint8_t *p_ss,
                     size_t ss_size);

  /**
   ****************************************************************************************************
   *                                            SHA-256()
   *
   * @brief   Compute a SHA-256 digest from the mesage passed in. This function encapsulates the init,
   *          update and finish stages of SHA-256 into one.
   *
   * @param   p_ctx[in]       Pointer to an initialized Vault context structure.
   *
   * @param   p_msg[in]       Buffer containing the message to run through SHA-256.
   *
   * @param   msg_size[in]    Size of the message to run through SHA-256.
   *
   * @param   p_digest[in]    Buffer to place the resulting digest in.
   *
   * @param   digest_size[in] Size of the digest buffer. Must be 32-bytes.
   *
   * @return  kOckamErrorNone on success.
   ****************************************************************************************************
   */

  OckamError (*Sha256)(void *ctx, uint8_t *p_msg, size_t msg_size, uint8_t *p_digest, size_t digest_size);

  /**
   ****************************************************************************************************
   *                                          Hkdf()
   *
   * @brief   Compute an HMAC-based key derivation to create a key of a desired size.
   *
   * @param   p_ctx[in]       Pointer to an initialized Vault context structure.
   *
   * @param   p_salt[in]      Buffer containing an optional salt value.
   *
   * @param   salt_size[in]   Size of the optional salt value for HKDF. If 0, p_salt must be 0.
   *
   * @param   p_ikm[in]       Buffer containing optional input key material.
   *
   * @param   ikm_size[in]    Size of the optional key material. If 0, p_ikm must be 0.
   *
   * @param   p_info[in]      Buffer containing optional context specific info.
   *
   * @param   info_size[in]   Size of the optional context specific info. If 0, p_info must be 0.
   *
   * @param   p_out[out]      Buffer to place the resulting key in. Can not be 0. Must be the size of
   *                          out_size.
   *
   * @param   out_size[in]    Size of the key to generate.
   *
   * @return  kOckamErrorNone on success.
   ****************************************************************************************************
   */

  OckamError (*Hkdf)(void *ctx, uint8_t *p_salt, size_t salt_size, uint8_t *p_ikm, size_t ikm_size, uint8_t *p_info,
                     size_t info_size, uint8_t *p_out, size_t out_size);

  /**
   ****************************************************************************************************
   *                                         AesGcmEncrypt()
   *
   * @brief   Encrypt a payload using AES-GCM.
   *
   * @param   p_ctx[in]       Pointer to an initialized Vault context structure.
   *
   * @param   p_key[in]       Buffer containing the key for AES-GCM encryption. Must not be 0.
   *
   * @param   key_size[in]    Size of AES-GCM encryption key in bytes.
   *
   * @param   p_iv[in]        Buffer containing input vector data for AES-GCM. Must not be 0.
   *
   * @param   iv_size[in]     Size of the input vector data. Can not be 0.
   *
   * @param   p_aad[in]       Optional buffer containing additional authentication data for AES-GCM.
   *
   * @param   aad_size[in]    Size of the additional authentication data. If 0, p_aad must be 0.
   *
   * @param   p_tag[out]      Buffer to place the resulting tag data from the AES-GCM encryption.
   *
   * @param   tag_size[in]    Size of the tag buffer. Must be 16 bytes.
   *
   * @param   p_input[in]     Buffer containing the material to encrypt using AES-GCM.
   *
   * @param   input_size[in]  Size of the input material to encrypt. Must match the output size.
   *
   * @param   p_output[out]   Buffer containing the encrypted hash from the AES-GCM encrypt.
   *
   * @param   output_size[in] Size of the output buffer. Must match the input size.
   *
   * @return  kOckamErrorNone on success.
   ****************************************************************************************************
   */

  OckamError (*AesGcmEncrypt)(void *ctx, uint8_t *p_key, size_t key_size, uint8_t *p_iv, size_t iv_size, uint8_t *p_aad,
                              size_t aad_size, uint8_t *p_tag, size_t tag_size, uint8_t *p_input, size_t input_size,
                              uint8_t *p_output, size_t output_size);

  /**
   ****************************************************************************************************
   *                                         AesGcmDecrypt()
   *
   * @brief   Decrypt a payload using AES-GCM.
   *
   * @param   p_ctx[in]       Pointer to an initialized Vault context structure.
   *
   * @param   p_key[in]       Buffer containing the key for AES-GCM encryption. Must not be 0.
   *
   * @param   key_size[in]    Size of AES-GCM encryption key in bytes.
   *
   * @param   p_iv[in]        Buffer containing input vector data for AES-GCM. Must not be 0.
   *
   * @param   iv_size[in]     Size of the input vector data. Can not be 0.
   *
   * @param   p_aad[in]       Optional buffer containing additional authentication data for AES-GCM.
   *
   * @param   aad_size[in]    Size of the additional authentication data. If 0, p_aad must be 0.
   *
   * @param   p_tag[in]       Buffer containing the tag data from the AES-GCM encrypt stage.
   *
   * @param   tag_size[in]    Size of the tag buffer. Must be 16 bytes.
   *
   * @param   p_input[in]     Buffer containing the encrypted hash to decrypt using AES-GCM.
   *
   * @param   input_size[in]  Size of the input material to decrypt. Must match the output size.
   *
   * @param   p_output[out]   Buffer to place the decrypted material in from the AES-GCM decrypt.
   *
   * @param   output_size[in] Size of the output buffer. Must match the input size.
   *
   * @return  kOckamErrorNone on success.
   ****************************************************************************************************
   */

  OckamError (*AesGcmDecrypt)(void *ctx, uint8_t *p_key, size_t key_size, uint8_t *p_iv, size_t iv_size, uint8_t *p_aad,
                              size_t aad_size, uint8_t *p_tag, size_t tag_size, uint8_t *p_input, size_t input_size,
                              uint8_t *p_output, size_t output_size);
} OckamVault;

#ifdef __cplusplus
}
#endif

/*
 ********************************************************************************************************
 * @}
 ********************************************************************************************************
 */

#endif
