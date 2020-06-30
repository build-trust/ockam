/**
 * @file    atecc608a.c
 * @brief   Ockam Vault Implementation for the ATECC608A
 */

#include "ockam/memory.h"
#include "ockam/mutex.h"
#include "ockam/vault.h"
#include "vault/impl.h"

#include "atecc608a.h"

#define VAULT_ATECC608A_NUM_SLOTS                  16u
#define VAULT_ATECC608A_DEVREV_MIN                 0x02600000      /* Minimum device rev from info                       */
#define VAULT_ATECC608A_DEVREV_MAX                 0x026000FF      /* Maximum device rev from info                       */
#define VAULT_ATECC608A_SS_SIZE                    32u             /* Size of the shared secret                          */
#define VAULT_ATECC608A_RAND_SIZE                  32u             /* Size of the random number generated                */
#define VAULT_ATECC608A_PUB_KEY_SIZE               64u             /* Size of public key                                 */
#define VAULT_ATECC608A_SLOT_WRITE_SIZE_MIN         4u             /* Smallest write possible is 4 bytes                 */
#define VAULT_ATECC608A_SLOT_WRITE_SIZE_MAX        32u             /* Largest write possible is 32 bytes                 */
#define VAULT_ATECC608A_SLOT_OFFSET_MAX             8u             /* Largest possible offset in slots                   */
#define VAULT_ATECC608A_CFG_LOCK_VALUE_UNLOCKED   0x55             /* Data and OTP are in an unlocked/configurable state */
#define VAULT_ATECC608A_CFG_LOCK_VALUE_LOCKED     0x00             /* Data and OTP are in a locked/unconfigurable state  */
#define VAULT_ATECC608A_CFG_LOCK_CONFIG_UNLOCKED  0x55             /* Config zone is in an unlocked/configurable state   */
#define VAULT_ATECC608A_CFG_LOCK_CONFIG_LOCKED    0x00             /* Config zone is in a locked/unconfigurable state    */
#define VAULT_ATECC608A_HMAC_HASH_SIZE             32u             /* HMAC hash output size                              */
#define VAULT_ATECC608A_AES_GCM_KEY_SIZE          128u             /* ATECC608A only supports AES GCM 128                */
#define VAULT_ATECC608A_AES_GCM_KEY_BLOCK           0u             /* AES Key starts at block 0 in slot 15               */
#define VAULT_ATECC608A_AEAD_AES_GCM_DECRYPT        0u             /* Signal common AES GCM function to decrypt          */
#define VAULT_ATECC608A_AEAD_AES_GCM_ENCRYPT        1u             /* Signal common AES GCM function to encrypt          */
#define VAULT_ATECC608A_AEAD_AES_GCM_IV_SIZE       12u
#define VAULT_ATECC608A_AEAD_AES_GCM_IV_OFFSET     10u

#define VAULT_ATECC608A_SLOT_GENKEY_MASK           0x2000
#define VAULT_ATECC608A_SLOT_PRIVWRITE_MASK        0x4000

#define VAULT_ATECC608A_KEY_REQRANDOM_MASK         0x40

#define VAULT_ATECC608A_KEY_TYPE_SHIFT             0x02
#define VAULT_ATECC608A_KEY_TYPE_MASK              0x1C
#define VAULT_ATECC608A_KEY_TYPE_P256              0x04
#define VAULT_ATECC608A_KEY_TYPE_AES               0x06
#define VAULT_ATECC608A_KEY_TYPE_BUFFER            0x07

#define VAULT_ATECC608A_SLOT_FEAT_NONE             0x00
#define VAULT_ATECC608A_SLOT_FEAT_IO_PROTECTION    0x01
#define VAULT_ATECC608A_SLOT_FEAT_PRIVKEY_GENERATE 0x02
#define VAULT_ATECC608A_SLOT_FEAT_PRIVKEY_WRITE    0x04
#define VAULT_ATECC608A_SLOT_FEAT_BUFFER           0x08
#define VAULT_ATECC608A_SLOT_FEAT_AESKEY           0x10

/**
 * @brief Configuration data for the ATECC608A
 */
#pragma pack(1)
typedef struct {            /*!< Byte(s): Description                             */
  uint8_t serial_num_0[4];  /*!< 0-3    : SN<0:3>                                 */
  uint32_t revision;        /*!< 4-7    : Revision Number                         */
  uint8_t serial_num_1[5];  /*!< 8-12   : SN<4:8>                                 */
  uint8_t aes_enable;       /*!< 13     : Bit 0: 0=AES disabled, 1=AES enabled    */
  uint8_t i2c_enable;       /*!< 14     : Bit 0: 0=SingleWire,1=I2C               */
  uint8_t reserved_1;       /*!< 15     : Reserved                                */
  uint8_t i2c_address;      /*!< 16     : I2C Address bits 7-1, bit 0 must be 0   */
  uint8_t reserved_2;       /*!< 17     : Reserved                                */
  uint8_t otp_mode;         /*!< 18     : Configures the OTP zone. RO or RW       */
  uint8_t chip_mode;        /*!< 19     : Bit 2-Watchdog,Bit 1-TTL,Bit 0-Selector */
  uint16_t slot_config[16]; /*!< 20-51  : 16 slot configurations                  */
  uint8_t counter_0[8];     /*!< 52-59  : Counter that can be connected to keys   */
  uint8_t counter_1[8];     /*!< 60-67  : Stand-alone counter                     */
  uint8_t last_key_use[16]; /*!< 68-83  : Control limited use for KeyID 15        */
  uint8_t user_extra;       /*!< 84     : 1 byte value updatedable after data lock*/
  uint8_t selector;         /*!< 85     : Selects device to be active after pause */
  uint8_t lock_value;       /*!< 86     : Lock state of the Data/OTP zone         */
  uint8_t lock_config;      /*!< 87     : Lock state of the configuration zone    */
  uint16_t slot_locked;     /*!< 88-89  : Bit for each slot. 0-Locked, 1-Unlocked */
  uint16_t rfu;             /*!< 90-91  : Must be 0                               */
  uint32_t x509_format;     /*!< 92-95  : Template length & public position config*/
  uint16_t key_config[16];  /*!< 96-127 : 16 key configurations                   */
} vault_atecc608a_cfg_t;
#pragma pack()

/**
 * @brief EEPROM slot configuration data
 */
typedef struct {
  ockam_vault_secret_t* secret;
  uint8_t               feat;
  uint8_t               req_random;
  uint8_t               write_key;
  uint8_t               read_key;
} vault_atecc608a_slot_cfg_t;

/**
 * @brief Context data for the ATECC608A
 */
typedef struct {
  ockam_memory_t*                       memory;
  ockam_mutex_t*                        mutex;
  ockam_mutex_lock_t                    lock;
  ockam_vault_atecc608a_io_protection_t io_protection;
  vault_atecc608a_cfg_t                 config;
  vault_atecc608a_slot_cfg_t            slot_config[VAULT_ATECC608A_NUM_SLOTS];
} vault_atecc608a_context_t;

/**
 * @brief Context data for the ATECC608A secrets
 */
typedef struct {
  uint16_t slot;
  uint8_t *buffer;
  size_t buffer_size;
} vault_atecc608a_secret_context_t;

uint16_t g_vault_atecc608a_slot_size[VAULT_ATECC608A_NUM_SLOTS] = {
  36, 36, 36, 36, 36, 36, 36, 36, 416, 72, 72, 72, 72, 72, 72, 72
};


ockam_error_t vault_atecc608a_deinit(ockam_vault_t* vault);

ockam_error_t vault_atecc608a_random(ockam_vault_t* vault, uint8_t* buffer, size_t buffer_size);

ockam_error_t vault_atecc608a_sha256(ockam_vault_t* vault,
                                     const uint8_t* input,
                                     size_t         input_length,
                                     uint8_t*       digest,
                                     size_t         digest_size,
                                     size_t*        digest_length);

ockam_error_t vault_atecc608a_secret_generate(ockam_vault_t*                         vault,
                                              ockam_vault_secret_t*                  secret,
                                              const ockam_vault_secret_attributes_t* attributes);

ockam_error_t vault_atecc608a_secret_import(ockam_vault_t*                         vault,
                                            ockam_vault_secret_t*                  secret,
                                            const ockam_vault_secret_attributes_t* attributes,
                                            const uint8_t*                         input,
                                            size_t                                 input_length);

ockam_error_t vault_atecc608a_secret_export(ockam_vault_t*        vault,
                                            ockam_vault_secret_t* secret,
                                            uint8_t*              output_buffer,
                                            size_t                output_buffer_size,
                                            size_t*               output_buffer_length);

ockam_error_t vault_atecc608a_secret_publickey_get(ockam_vault_t*        vault,
                                                   ockam_vault_secret_t* secret,
                                                   uint8_t*              output_buffer,
                                                   size_t                output_buffer_size,
                                                   size_t*               output_buffer_length);

ockam_error_t vault_atecc608a_secret_attributes_get(ockam_vault_t*                   vault,
                                                    ockam_vault_secret_t*            secret,
                                                    ockam_vault_secret_attributes_t* attributes);

ockam_error_t vault_atecc608a_secret_type_set(ockam_vault_t*            vault,
                                              ockam_vault_secret_t*     secret,
                                              ockam_vault_secret_type_t type);

ockam_error_t vault_atecc608a_secret_destroy(ockam_vault_t* vault, ockam_vault_secret_t* secret);

ockam_error_t vault_atecc608a_ecdh(ockam_vault_t*        vault,
                                   ockam_vault_secret_t* privatekey,
                                   const uint8_t*        peer_publickey,
                                   size_t                peer_publickey_length,
                                   ockam_vault_secret_t* shared_secret);

ockam_error_t vault_atecc608a_hkdf_sha256(ockam_vault_t*        vault,
                                          ockam_vault_secret_t* salt,
                                          ockam_vault_secret_t* input_key_material,
                                          uint8_t               derived_outputs_count,
                                          ockam_vault_secret_t* derived_outputs);

ockam_error_t vault_atecc608a_aead_aes_gcm_encrypt(ockam_vault_t*        vault,
                                                   ockam_vault_secret_t* key,
                                                   uint16_t              nonce,
                                                   const uint8_t*        additional_data,
                                                   size_t                additional_data_length,
                                                   const uint8_t*        plaintext,
                                                   size_t                plaintext_length,
                                                   uint8_t*              ciphertext_and_tag,
                                                   size_t                ciphertext_and_tag_size,
                                                   size_t*               ciphertext_and_tag_length);

ockam_error_t vault_atecc608a_aead_aes_gcm_decrypt(ockam_vault_t*        vault,
                                                   ockam_vault_secret_t* key,
                                                   uint16_t              nonce,
                                                   const uint8_t*        additional_data,
                                                   size_t                additional_data_length,
                                                   const uint8_t*        ciphertext_and_tag,
                                                   size_t                ciphertext_and_tag_length,
                                                   uint8_t*              plaintext,
                                                   size_t                plaintext_size,
                                                   size_t*               plaintext_length);

ockam_error_t atecc608a_hkdf_extract(vault_atecc608a_context_t*        context,
                                     vault_atecc608a_secret_context_t* salt,
                                     vault_atecc608a_secret_context_t* ikm,
                                     uint16_t*                         prk_slot);

ockam_error_t atecc608a_hkdf_expand(vault_atecc608a_context_t* context,
                                    ockam_vault_secret_t*      outputs,
                                    uint8_t                    outputs_count,
                                    uint16_t                   prk_slot);

ockam_error_t atecc608a_aead_aes_gcm(ockam_vault_t*        vault,
                                     int                   encrypt,
                                     ockam_vault_secret_t* key,
                                     uint16_t              nonce,
                                     const uint8_t*        additional_data,
                                     size_t                additional_data_length,
                                     const uint8_t*        ciphertext_and_tag,
                                     size_t                ciphertext_and_tag_length,
                                     uint8_t*              plaintext,
                                     size_t                plaintext_size,
                                     size_t*               plaintext_length);

ockam_vault_dispatch_table_t vault_atecc608a_dispatch_table = {
  &vault_atecc608a_deinit,
  &vault_atecc608a_random,
  &vault_atecc608a_sha256,
  &vault_atecc608a_secret_generate,
  &vault_atecc608a_secret_import,
  &vault_atecc608a_secret_export,
  &vault_atecc608a_secret_publickey_get,
  &vault_atecc608a_secret_attributes_get,
  &vault_atecc608a_secret_type_set,
  &vault_atecc608a_secret_destroy,
  &vault_atecc608a_ecdh,
  &vault_atecc608a_hkdf_sha256,
  &vault_atecc608a_aead_aes_gcm_encrypt,
  &vault_atecc608a_aead_aes_gcm_decrypt,
};

ockam_error_t ockam_vault_atecc608a_init(ockam_vault_t* vault, ockam_vault_atecc608a_attributes_t* attributes)
{
  ockam_error_t              error   = OCKAM_ERROR_NONE;
  ATCA_STATUS                status  = ATCA_SUCCESS;
  vault_atecc608a_context_t* context = 0;
  uint8_t                    i       = 0;

  if((vault == 0) || (attributes == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if(attributes->memory == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_ATTRIBUTES;
    goto exit;
  }

  error = ockam_memory_alloc_zeroed(attributes->memory, (void**) &context, sizeof(vault_atecc608a_context_t));
  if(error != OCKAM_ERROR_NONE) {
    goto exit;
  }

  context->memory = attributes->memory;

  if((attributes->io_protection->key == 0) ||
     (attributes->io_protection->slot > VAULT_ATECC608A_NUM_SLOTS) ||
     (attributes->io_protection->key_size > g_vault_atecc608a_slot_size[attributes->io_protection->slot])) {
    error = OCKAM_VAULT_ERROR_INVALID_ATTRIBUTES;
    goto exit;
  }

  error = ockam_memory_copy(context->memory,
                            &(context->io_protection),
                            attributes->io_protection,
                            sizeof(ockam_vault_atecc608a_io_protection_t));
  if(error != OCKAM_ERROR_NONE) {
    goto exit;
  }

  if(attributes->mutex != 0) {
    context->mutex = attributes->mutex;

    error = ockam_mutex_create(context->mutex, context->lock);
    if(error != OCKAM_ERROR_NONE) {
      goto exit;
    }
  }

  status = atcab_init(attributes->atca_iface_cfg);
  if (status != ATCA_SUCCESS) {
    error = OCKAM_VAULT_ERROR_INIT_FAIL;
    goto exit;
  }

  status = atcab_read_config_zone((uint8_t *)&(context->config));
  if (status != ATCA_SUCCESS) {
    error = OCKAM_VAULT_ERROR_INIT_FAIL;
    goto exit;
  }

  if ((context->config.revision < VAULT_ATECC608A_DEVREV_MIN) ||
      (context->config.revision > VAULT_ATECC608A_DEVREV_MAX)) {
    error = OCKAM_VAULT_ERROR_INIT_FAIL;
    goto exit;
  }

  if ((context->config.lock_config != VAULT_ATECC608A_CFG_LOCK_CONFIG_LOCKED) ||
      (context->config.lock_value != VAULT_ATECC608A_CFG_LOCK_CONFIG_LOCKED)) {
    error = OCKAM_VAULT_ERROR_INIT_FAIL;
    goto exit;
  }

  if(context->config.aes_enable == 0) {
    error = OCKAM_VAULT_ERROR_INIT_FAIL;
    goto exit;
  }

  for(i = 0; i < VAULT_ATECC608A_NUM_SLOTS; i++) {

    context->slot_config[i].req_random = (context->config.key_config[i] & VAULT_ATECC608A_KEY_REQRANDOM_MASK);

    switch((context->config.key_config[i] & VAULT_ATECC608A_KEY_TYPE_MASK) >> VAULT_ATECC608A_KEY_TYPE_SHIFT)
    {
      case VAULT_ATECC608A_KEY_TYPE_P256:

        if(context->config.slot_config[i] & VAULT_ATECC608A_SLOT_GENKEY_MASK) {
          context->slot_config[i].feat |= VAULT_ATECC608A_SLOT_FEAT_PRIVKEY_GENERATE;
        }

        if(context->config.slot_config[i] & VAULT_ATECC608A_SLOT_PRIVWRITE_MASK) {
          context->slot_config[i].feat |= VAULT_ATECC608A_SLOT_FEAT_PRIVKEY_WRITE;
        }
        break;

      case VAULT_ATECC608A_KEY_TYPE_AES:
          if(i == 15) { //TODO Determine why slots 13 & 14 produce invalid results.
            context->slot_config[i].feat |= VAULT_ATECC608A_SLOT_FEAT_AESKEY;
          } else {
            context->slot_config[i].feat |= VAULT_ATECC608A_SLOT_FEAT_NONE;
          }
        break;

      case VAULT_ATECC608A_KEY_TYPE_BUFFER:
        if(i > 8) {
          context->slot_config[i].feat |= VAULT_ATECC608A_SLOT_FEAT_BUFFER;
        } else {
          context->slot_config[i].feat |= VAULT_ATECC608A_SLOT_FEAT_NONE;
        }
        break;

      default:
        break;
    }
  }

  status = atcab_write_bytes_zone(ATCA_ZONE_DATA,
                                  context->io_protection.slot,
                                  0,
                                  context->io_protection.key,
                                  context->io_protection.key_size);
  if (status != ATCA_SUCCESS) {
    error = OCKAM_VAULT_ERROR_INIT_FAIL;
    goto exit;
  }


  vault->dispatch = &vault_atecc608a_dispatch_table;
  vault->impl_context  = context;

exit:
  return error;
}

/**
 ********************************************************************************************************
 *                                      vault_atecc608a_deinit()
 ********************************************************************************************************
 */

ockam_error_t vault_atecc608a_deinit(ockam_vault_t* vault)
{
  ockam_error_t              error   = OCKAM_ERROR_NONE;
  ATCA_STATUS                status  = ATCA_SUCCESS;
  vault_atecc608a_context_t* context = 0;

  if ((vault == 0) || (vault->impl_context == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  context = (vault_atecc608a_context_t*) vault->impl_context;

  ockam_mutex_destroy(context->mutex, context->lock);

  error = ockam_memory_free(context->memory, context, sizeof(vault_atecc608a_context_t));

  vault->dispatch = 0;
  vault->impl_context  = 0;

exit:
  return error;
}

/**
 ********************************************************************************************************
 *                                       vault_atecc608a_random()
 ********************************************************************************************************
 */

ockam_error_t vault_atecc608a_random(ockam_vault_t* vault, uint8_t* buffer, size_t buffer_size)
{
  ockam_error_t              error      = OCKAM_ERROR_NONE;
  ockam_error_t              exit_error = OCKAM_ERROR_NONE;
  ATCA_STATUS                status     = ATCA_SUCCESS;
  vault_atecc608a_context_t* context    = 0;

  if ((vault == 0) || (vault->impl_context == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  context = (vault_atecc608a_context_t*) vault->impl_context;

  if(buffer == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if (buffer_size != VAULT_ATECC608A_RAND_SIZE) {
    error = OCKAM_VAULT_ERROR_INVALID_SIZE;
    goto exit;
  }

  if(context->mutex) {
    error = ockam_mutex_lock(context->mutex, context->lock);
    if(error != OCKAM_ERROR_NONE) {
      goto exit;
    }
  }

  status = atcab_random(buffer);
  if (status != ATCA_SUCCESS) {
    error = OCKAM_VAULT_ERROR_RANDOM_FAIL;;
    goto exit;
  }

exit:
  if(context->mutex) {
    exit_error = ockam_mutex_unlock(context->mutex, context->lock);
    if(error == OCKAM_ERROR_NONE) {
      error = exit_error;
    }
  }

  return error;
}

/**
 ********************************************************************************************************
 *                                       vault_atecc608a_sha256()
 ********************************************************************************************************
 */

ockam_error_t vault_atecc608a_sha256(ockam_vault_t* vault,
                                     const uint8_t* input,
                                     size_t         input_length,
                                     uint8_t*       digest,
                                     size_t         digest_size,
                                     size_t*        digest_length)
{
  ockam_error_t              error      = OCKAM_ERROR_NONE;
  ockam_error_t              exit_error = OCKAM_ERROR_NONE;
  ATCA_STATUS                status     = ATCA_SUCCESS;
  vault_atecc608a_context_t* context    = 0;

  if ((vault == 0) || (vault->impl_context == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  context = (vault_atecc608a_context_t*) vault->impl_context;

  if(digest == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if(digest_size != OCKAM_VAULT_SHA256_DIGEST_LENGTH) {
    error = OCKAM_VAULT_ERROR_INVALID_SIZE;
    goto exit;
  }

  if(context->mutex) {
    error = ockam_mutex_lock(context->mutex, context->lock);
    if(error != OCKAM_ERROR_NONE) {
      goto exit;
    }
  }

  status = atcab_sha(input_length, input, digest);
  if (status != ATCA_SUCCESS) {
    error = OCKAM_VAULT_ERROR_SHA256_FAIL;
    goto exit;
  }

  *digest_length = digest_size;

exit:
  if(context->mutex) {
    exit_error = ockam_mutex_unlock(context->mutex, context->lock);
    if(error == OCKAM_ERROR_NONE) {
      error = exit_error;
    }
  }

  return error;
}

/**
 ********************************************************************************************************
 *                                    vault_atecc608a_secret_generate()
 ********************************************************************************************************
 */

ockam_error_t vault_atecc608a_secret_generate(ockam_vault_t*                         vault,
                                              ockam_vault_secret_t*                  secret,
                                              const ockam_vault_secret_attributes_t* attributes)
{
  ockam_error_t                     error                           = OCKAM_ERROR_NONE;
  ockam_error_t                     exit_error                      = OCKAM_ERROR_NONE;
  ATCA_STATUS                       status                          = ATCA_SUCCESS;
  vault_atecc608a_context_t*        context                         = 0;
  vault_atecc608a_secret_context_t* secret_ctx                      = 0;
  uint8_t                           rand[VAULT_ATECC608A_RAND_SIZE] = {0};
  uint8_t                           slot                            = 0;

  if ((vault == 0) || (vault->impl_context == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  context = (vault_atecc608a_context_t*) vault->impl_context;

  if(secret == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_SECRET;
    goto exit;
  }


  if(attributes == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if(attributes->type != OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY) {
    error = OCKAM_VAULT_ERROR_INVALID_ATTRIBUTES;
    goto exit;
  }

  if(context->mutex) {
    error = ockam_mutex_lock(context->mutex, context->lock);
    if(error != OCKAM_ERROR_NONE) {
      goto exit;
    }
  }

  for(slot = 0; slot <= VAULT_ATECC608A_NUM_SLOTS; slot++) {
    if(slot == VAULT_ATECC608A_NUM_SLOTS) {
      error = OCKAM_VAULT_ERROR_SECRET_GENERATE_FAIL;
      goto exit;
    }

    if((context->slot_config[slot].secret == 0) &&
       (context->slot_config[slot].feat & VAULT_ATECC608A_SLOT_FEAT_PRIVKEY_GENERATE)) {
      break;
    }
  }

  if(context->slot_config[slot].req_random) {
    status = atcab_random(&rand[0]); /* Get a random number from the ATECC608A             */
    if (status != ATCA_SUCCESS) {    /* before a genkey operation.                         */
      error = OCKAM_VAULT_ERROR_SECRET_GENERATE_FAIL;
      goto exit;
    }

    status = atcab_nonce((const uint8_t *)&rand[0]); /* Feed the random number back into the ATECC608A     */
    if (status != ATCA_SUCCESS) {                    /* before a genkey operation.                         */
      error = OCKAM_VAULT_ERROR_SECRET_GENERATE_FAIL;
      goto exit;
    }
  }

  status = atcab_genkey(slot, 0);
  if (status != ATCA_SUCCESS) {
    error = OCKAM_VAULT_ERROR_SECRET_GENERATE_FAIL;
    goto exit;
  }

  error = ockam_memory_alloc_zeroed(context->memory,
                                    (void**) &(secret_ctx),
                                    sizeof(vault_atecc608a_secret_context_t));
  if(error != OCKAM_ERROR_NONE) {
    goto exit;
  }

  secret_ctx->slot = slot;
  secret->context  = secret_ctx;

  ockam_memory_copy(context->memory, &(secret->attributes), attributes, sizeof(ockam_vault_secret_attributes_t));
  context->slot_config[slot].secret = secret;


exit:
  if(context->mutex) {
    exit_error = ockam_mutex_unlock(context->mutex, context->lock);
    if(error == OCKAM_ERROR_NONE) {
      error = exit_error;
    }
  }

  return error;
}

/**
 ********************************************************************************************************
 *                                  vault_atecc608a_secret_import()
 ********************************************************************************************************
 */

ockam_error_t vault_atecc608a_secret_import(ockam_vault_t*                         vault,
                                            ockam_vault_secret_t*                  secret,
                                            const ockam_vault_secret_attributes_t* attributes,
                                            const uint8_t*                         input,
                                            size_t                                 input_length)
{
  ockam_error_t                     error                            = OCKAM_ERROR_NONE;
  ockam_error_t                     exit_error                       = OCKAM_ERROR_NONE;
  vault_atecc608a_context_t*        context                          = 0;
  vault_atecc608a_secret_context_t* secret_ctx                       = 0;
  uint8_t                           slot                             = 0;
  ATCA_STATUS                       status                           = ATCA_SUCCESS;
  uint8_t*                          buffer                           = 0;
  uint8_t                           nonce[VAULT_ATECC608A_RAND_SIZE] = {0};

  if ((vault == 0) || (vault->impl_context == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  context = (vault_atecc608a_context_t*) vault->impl_context;

  if((secret == 0) || (secret->context != 0) || (attributes == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  if((attributes->type == OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY) ||       //TODO change this when configured to allow
     (attributes->type == OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY) || // private key import for testing
     (attributes->type == OCKAM_VAULT_SECRET_TYPE_AES256_KEY)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  error = ockam_memory_alloc_zeroed(context->memory,
                                    (void**) &secret_ctx,
                                    sizeof(vault_atecc608a_secret_context_t));
  if(error != OCKAM_ERROR_NONE) {
    goto exit;
  }

  if(context->mutex) {
    error = ockam_mutex_lock(context->mutex, context->lock);
    if(error != OCKAM_ERROR_NONE) {
      goto exit;
    }
  }

  if((attributes->type == OCKAM_VAULT_SECRET_TYPE_AES128_KEY) ||
     (attributes->type == OCKAM_VAULT_SECRET_TYPE_BUFFER)) {

    error = ockam_memory_alloc_zeroed(context->memory,
                                      (void**) &(secret_ctx->buffer),
                                      input_length);
    if(error != OCKAM_ERROR_NONE) {
      goto exit;
    }

    error = ockam_memory_copy(context->memory,
                              secret_ctx->buffer,
                              input,
                              input_length);

    secret_ctx->buffer_size = input_length;
  }

  else if(attributes->type == OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY) {

    if(input_length != OCKAM_VAULT_P256_PRIVATEKEY_LENGTH) {
      error = OCKAM_VAULT_ERROR_INVALID_SIZE;
      goto exit;
    }

    for(slot = 0; slot <= VAULT_ATECC608A_NUM_SLOTS; slot++) {
      if(slot == VAULT_ATECC608A_NUM_SLOTS) {
        error = OCKAM_VAULT_ERROR_SECRET_IMPORT_FAIL;
        goto exit;
      }

      if((context->slot_config[slot].secret == 0) &&
         (context->slot_config[slot].feat & VAULT_ATECC608A_SLOT_FEAT_PRIVKEY_WRITE)) {
        break;
      }
    }

    status = atcab_random(&nonce[0]);
    if (status != ATCA_SUCCESS) {
      error = OCKAM_VAULT_ERROR_SECRET_GENERATE_FAIL;
      goto exit;
    }

    status = atcab_nonce((const uint8_t *)&nonce[0]);
    if (status != ATCA_SUCCESS) {
      error = OCKAM_VAULT_ERROR_ECDH_FAIL;
      goto exit;
    }

    status = atcab_write_enc(slot, 0, input, context->io_protection.key, context->io_protection.key_size, &nonce[0]);
    if(status != ATCA_SUCCESS) {
      error = OCKAM_VAULT_ERROR_SECRET_IMPORT_FAIL;
      goto exit;
    }
  }

  ockam_memory_copy(context->memory, &(secret->attributes), attributes, sizeof(ockam_vault_secret_attributes_t));
  secret->context = secret_ctx;

  secret_ctx->slot = slot;
  context->slot_config[slot].secret = secret;

exit:

  if((error != OCKAM_ERROR_NONE) && (secret_ctx != 0)) {
    if(secret_ctx->buffer != 0) {
      ockam_memory_free(context->memory, secret_ctx->buffer, input_length);
    }
    ockam_memory_free(context->memory, secret_ctx, sizeof(vault_atecc608a_secret_context_t));
  }

  if(context->mutex) {
    exit_error = ockam_mutex_unlock(context->mutex, context->lock);
    if(error == OCKAM_ERROR_NONE) {
      error = exit_error;
    }
  }

  return error;
}

/**
 ********************************************************************************************************
 *                                  vault_atecc608a_secret_export()
 ********************************************************************************************************
 */

ockam_error_t vault_atecc608a_secret_export(ockam_vault_t*        vault,
                                            ockam_vault_secret_t* secret,
                                            uint8_t*              output_buffer,
                                            size_t                output_buffer_size,
                                            size_t*               output_buffer_length)
{
  ockam_error_t                     error       = OCKAM_ERROR_NONE;
  ockam_error_t                     exit_error  = OCKAM_ERROR_NONE;
  vault_atecc608a_context_t*        context     = 0;
  vault_atecc608a_secret_context_t* secret_ctx  = 0;
  ATCA_STATUS                       status      = ATCA_SUCCESS;
  uint8_t*                          buffer      = 0;

  if ((vault == 0) || (vault->impl_context == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  context = (vault_atecc608a_context_t*) vault->impl_context;

  if((secret == 0) || (secret->context == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  secret_ctx = (vault_atecc608a_secret_context_t*) secret->context;

  if((output_buffer == 0) || (output_buffer_size == 0) || (output_buffer_length == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  if((secret->attributes.type == OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY) ||
     (secret->attributes.type == OCKAM_VAULT_SECRET_TYPE_CURVE25519_PRIVATEKEY) ||
     (secret->attributes.type == OCKAM_VAULT_SECRET_TYPE_AES256_KEY)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if(context->mutex) {
    error = ockam_mutex_lock(context->mutex, context->lock);
    if(error != OCKAM_ERROR_NONE) {
      goto exit;
    }
  }

  if(output_buffer_size < secret_ctx->buffer_size) {
    error = OCKAM_VAULT_ERROR_INVALID_SIZE;
    goto exit;
  }

  error = ockam_memory_copy(context->memory,
                            output_buffer,
                            secret_ctx->buffer,
                            secret_ctx->buffer_size);
  if(error != OCKAM_ERROR_NONE) {
    goto exit;
  }

  *output_buffer_length = secret_ctx->buffer_size;

exit:
  if(context->mutex) {
    exit_error = ockam_mutex_unlock(context->mutex, context->lock);
    if(error == OCKAM_ERROR_NONE) {
      error = exit_error;
    }
  }
}

/**
 ********************************************************************************************************
 *                             vault_atecc608a_secret_attributes_get()
 ********************************************************************************************************
 */

ockam_error_t vault_atecc608a_secret_attributes_get(ockam_vault_t*                   vault,
                                                    ockam_vault_secret_t*            secret,
                                                    ockam_vault_secret_attributes_t* attributes)
{
  ockam_error_t              error   = OCKAM_ERROR_NONE;
  vault_atecc608a_context_t* context = 0;
  size_t                     size    = 0;

  if ((vault == 0) || (secret == 0) || (attributes == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if (vault->impl_context == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  context = (vault_atecc608a_context_t*) vault->impl_context;

  error = ockam_memory_copy(context->memory,
                            attributes,
                            &(secret->attributes),
                            sizeof(ockam_vault_secret_attributes_t));

exit:
  return error;
}

/**
 ********************************************************************************************************
 *                             vault_atecc608a_secret_type_set()
 ********************************************************************************************************
 */

ockam_error_t vault_atecc608a_secret_type_set(ockam_vault_t*            vault,
                                              ockam_vault_secret_t*     secret,
                                              ockam_vault_secret_type_t type)
{
  ockam_error_t                     error      = OCKAM_ERROR_NONE;
  vault_atecc608a_context_t*        ctx        = 0;
  vault_atecc608a_secret_context_t* secret_ctx = 0;

  if ((vault == 0) || (secret == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if ((secret->attributes.type != OCKAM_VAULT_SECRET_TYPE_BUFFER) &&
      (secret->attributes.type != OCKAM_VAULT_SECRET_TYPE_AES128_KEY)) {
    error = OCKAM_VAULT_ERROR_INVALID_SECRET_TYPE;
    goto exit;
  }

  if (secret->context == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  secret_ctx = (vault_atecc608a_secret_context_t*) secret->context;

  if (type == OCKAM_VAULT_SECRET_TYPE_AES128_KEY) {

    secret->attributes.type   = type;
    secret->attributes.length = OCKAM_VAULT_AES128_KEY_LENGTH;
  }

exit:
  return error;
}

/**
 ********************************************************************************************************
 *                           vault_atecc608a_secret_publickey_get()
 ********************************************************************************************************
 */

ockam_error_t vault_atecc608a_secret_destroy(ockam_vault_t* vault, ockam_vault_secret_t* secret)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  return error;
}

/**
 ********************************************************************************************************
 *                           vault_atecc608a_secret_publickey_get()
 ********************************************************************************************************
 */

ockam_error_t vault_atecc608a_secret_publickey_get(ockam_vault_t*        vault,
                                                   ockam_vault_secret_t* secret,
                                                   uint8_t*              output_buffer,
                                                   size_t                output_buffer_size,
                                                   size_t*               output_buffer_length)
{
  ockam_error_t                     error                           = OCKAM_ERROR_NONE;
  ockam_error_t                     exit_error                      = OCKAM_ERROR_NONE;
  ATCA_STATUS                       status                          = ATCA_SUCCESS;
  vault_atecc608a_context_t*        context                         = 0;
  vault_atecc608a_secret_context_t* secret_ctx                      = 0;
  uint8_t                           rand[VAULT_ATECC608A_RAND_SIZE] = {0};
  uint8_t                           slot                            = 0;

  if ((vault == 0) || (vault->impl_context == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  context = (vault_atecc608a_context_t*) vault->impl_context;

  if(secret == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_SECRET;
    goto exit;
  }

  secret_ctx = (vault_atecc608a_secret_context_t*) secret->context;

  if((output_buffer == 0) || (output_buffer_length == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if(output_buffer_size < OCKAM_VAULT_P256_PUBLICKEY_LENGTH) {
    error = OCKAM_VAULT_ERROR_INVALID_SIZE;
    goto exit;
  }

  if(context->mutex) {
    error = ockam_mutex_lock(context->mutex, context->lock);
    if(error != OCKAM_ERROR_NONE) {
      goto exit;
    }
  }

  status = atcab_get_pubkey(secret_ctx->slot, output_buffer);
  if (status != ATCA_SUCCESS) {
    error = OCKAM_VAULT_ERROR_PUBLIC_KEY_FAIL;
  }

  *output_buffer_length = OCKAM_VAULT_P256_PUBLICKEY_LENGTH;

exit:
  if(context->mutex) {
    exit_error = ockam_mutex_unlock(context->mutex, context->lock);
    if(error == OCKAM_ERROR_NONE) {
      error = exit_error;
    }
  }

  return error;
}

/**
 ********************************************************************************************************
 *                                       vault_atecc608a_ecdh()
 ********************************************************************************************************
 */

ockam_error_t vault_atecc608a_ecdh(ockam_vault_t*        vault,
                                   ockam_vault_secret_t* privatekey,
                                   const uint8_t*        peer_publickey,
                                   size_t                peer_publickey_length,
                                   ockam_vault_secret_t* shared_secret)
{
  ockam_error_t                     error                           = OCKAM_ERROR_NONE;
  ockam_error_t                     exit_error                      = OCKAM_ERROR_NONE;
  ATCA_STATUS                       status                          = ATCA_SUCCESS;
  uint8_t                           rand[VAULT_ATECC608A_RAND_SIZE] = {0};
  vault_atecc608a_context_t*        context                         = 0;
  vault_atecc608a_secret_context_t* privatekey_ctx                  = 0;
  vault_atecc608a_secret_context_t* shared_secret_ctx               = 0;

  if ((vault == 0) || (vault->impl_context == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  context = (vault_atecc608a_context_t*) vault->impl_context;

  if((privatekey == 0) ||
     (privatekey->attributes.type != OCKAM_VAULT_SECRET_TYPE_P256_PRIVATEKEY) ||
     (shared_secret == 0) ||
     (shared_secret->context != 0))
  {
    error = OCKAM_VAULT_ERROR_INVALID_SECRET_TYPE;
    goto exit;
  }

  privatekey_ctx    = (vault_atecc608a_secret_context_t*) privatekey->context;

  if((privatekey == 0) || (peer_publickey == 0) || (shared_secret == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  if(peer_publickey_length != OCKAM_VAULT_P256_PUBLICKEY_LENGTH) {
    error = OCKAM_VAULT_ERROR_INVALID_SIZE;
    goto exit;
  }

  error = ockam_memory_alloc_zeroed(context->memory,
                                    (void**) &(shared_secret_ctx),
                                    sizeof(vault_atecc608a_secret_context_t));
  if(error != OCKAM_ERROR_NONE) {
    goto exit;
  }

  error = ockam_memory_alloc_zeroed(context->memory,
                                    (void**) &(shared_secret_ctx->buffer),
                                    OCKAM_VAULT_SHARED_SECRET_LENGTH);
  if(error != OCKAM_ERROR_NONE) {
    goto exit;
  }

  shared_secret_ctx->buffer_size = OCKAM_VAULT_SHARED_SECRET_LENGTH;

  if(context->mutex) {
    error = ockam_mutex_lock(context->mutex, context->lock);
    if(error != OCKAM_ERROR_NONE) {
      goto exit;
    }
  }

  // TODO expand public key if compressed

  status = atcab_random(&rand[0]);
  if (status != ATCA_SUCCESS) {
    error = OCKAM_VAULT_ERROR_ECDH_FAIL;
    goto exit;
  }

  status = atcab_nonce((const uint8_t *)&rand[0]);
  if (status != ATCA_SUCCESS) {
    error = OCKAM_VAULT_ERROR_ECDH_FAIL;
    goto exit;
  }

  status = atcab_ecdh(privatekey_ctx->slot, peer_publickey, shared_secret_ctx->buffer);
  if (status != ATCA_SUCCESS) {
    error = OCKAM_VAULT_ERROR_ECDH_FAIL;
    goto exit;
  }

  shared_secret->context = shared_secret_ctx;


exit:

  if((error != OCKAM_ERROR_NONE) && (shared_secret_ctx != 0)) {
    if(shared_secret_ctx->buffer != 0) {
      ockam_memory_free(context->memory, shared_secret_ctx->buffer, OCKAM_VAULT_SHARED_SECRET_LENGTH);
    }
  }

  if(context->mutex) {
    exit_error = ockam_mutex_unlock(context->mutex, context->lock);
    if(error == OCKAM_ERROR_NONE) {
      error = exit_error;
    }
  }

  return error;
}

/**
 ********************************************************************************************************
 *                                       vault_atecc608a_hkdf_sha256()
 ********************************************************************************************************
 */

ockam_error_t vault_atecc608a_hkdf_sha256(ockam_vault_t*        vault,
                                          ockam_vault_secret_t* salt,
                                          ockam_vault_secret_t* input_key_material,
                                          uint8_t               derived_outputs_count,
                                          ockam_vault_secret_t* derived_outputs)
{
  ockam_error_t                     error                               = OCKAM_ERROR_NONE;
  ockam_error_t                     exit_error                          = OCKAM_ERROR_NONE;
  ATCA_STATUS                       status                              = ATCA_SUCCESS;
  uint8_t                           rand[VAULT_ATECC608A_RAND_SIZE]     = {0};
  uint8_t                           prk[VAULT_ATECC608A_HMAC_HASH_SIZE] = {0};
  ockam_vault_secret_t*             outputs                             = 0;
  vault_atecc608a_context_t*        context                             = 0;
  vault_atecc608a_secret_context_t* salt_ctx                            = 0;
  vault_atecc608a_secret_context_t* ikm_ctx                             = 0;
  uint16_t                          prk_slot                            = 0;

  if ((vault == 0) || (vault->impl_context == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  context = (vault_atecc608a_context_t*) vault->impl_context;

  if((salt == 0) || (input_key_material == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  salt_ctx = (vault_atecc608a_secret_context_t*) salt->context;
  ikm_ctx  = (vault_atecc608a_secret_context_t*) input_key_material->context;

  if(context->mutex) {
    error = ockam_mutex_lock(context->mutex, context->lock);
    if(error != OCKAM_ERROR_NONE) {
      goto exit;
    }
  }

  error = atecc608a_hkdf_extract(context,
                                 salt_ctx,
                                 ikm_ctx,
                                 &prk_slot);
  if(error != OCKAM_ERROR_NONE) {
    goto exit;
  }

  error = atecc608a_hkdf_expand(context,         /* Expand stage of HKDF. Uses the PRK from extract    */
                                derived_outputs, /* and outputs the key at the desired output size.    */
                                derived_outputs_count,
                                prk_slot);
exit:

  if(context->mutex) {
    exit_error = ockam_mutex_unlock(context->mutex, context->lock);
    if(error == OCKAM_ERROR_NONE) {
      error = exit_error;
    }
  }

  return error;
}


/**
 ********************************************************************************************************
 *                               vault_atecc608a_aead_aes_gcm_encrypt()
 ********************************************************************************************************
 */

ockam_error_t vault_atecc608a_aead_aes_gcm_encrypt(ockam_vault_t*        vault,
                                                   ockam_vault_secret_t* key,
                                                   uint16_t              nonce,
                                                   const uint8_t*        additional_data,
                                                   size_t                additional_data_length,
                                                   const uint8_t*        plaintext,
                                                   size_t                plaintext_length,
                                                   uint8_t*              ciphertext_and_tag,
                                                   size_t                ciphertext_and_tag_size,
                                                   size_t*               ciphertext_and_tag_length)
{
  return atecc608a_aead_aes_gcm(vault, VAULT_ATECC608A_AEAD_AES_GCM_ENCRYPT, key, nonce, additional_data,
                                additional_data_length, plaintext, plaintext_length, ciphertext_and_tag,
                                ciphertext_and_tag_size, ciphertext_and_tag_length);
}

/**
 ********************************************************************************************************
 *                               vault_atecc608a_aead_aes_gcm_decrypt()
 ********************************************************************************************************
 */

ockam_error_t vault_atecc608a_aead_aes_gcm_decrypt(ockam_vault_t*        vault,
                                                   ockam_vault_secret_t* key,
                                                   uint16_t              nonce,
                                                   const uint8_t*        additional_data,
                                                   size_t                additional_data_length,
                                                   const uint8_t*        ciphertext_and_tag,
                                                   size_t                ciphertext_and_tag_length,
                                                   uint8_t*              plaintext,
                                                   size_t                plaintext_size,
                                                   size_t*               plaintext_length)
{
  return atecc608a_aead_aes_gcm(vault, VAULT_ATECC608A_AEAD_AES_GCM_DECRYPT, key, nonce, additional_data,
                                additional_data_length, ciphertext_and_tag, ciphertext_and_tag_length,
                                plaintext, plaintext_size, plaintext_length);
}

/*
 ********************************************************************************************************
 *                                            LOCAL FUNCTIONS                                           *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                      atecc608a_hkdf_extract()
 ********************************************************************************************************
 */

ockam_error_t atecc608a_hkdf_extract(vault_atecc608a_context_t*        context,
                                     vault_atecc608a_secret_context_t* salt,
                                     vault_atecc608a_secret_context_t* ikm,
                                     uint16_t*                         prk_slot)
{
  ockam_error_t error                                         = OCKAM_ERROR_NONE;
  ATCA_STATUS   status                                        = ATCA_SUCCESS;
  uint16_t      slot                                          = 0;
  uint8_t       tmpkey[OCKAM_VAULT_HKDF_SHA256_OUTPUT_LENGTH] = {0};

  if((context == 0) || (salt == 0) || (ikm == 0)) {
    error = OCKAM_VAULT_ERROR_HKDF_SHA256_FAIL;
    goto exit;
  }

  for(slot = 0; slot <= VAULT_ATECC608A_NUM_SLOTS; slot++) {
    if(slot == VAULT_ATECC608A_NUM_SLOTS) {
      error = OCKAM_VAULT_ERROR_HKDF_SHA256_FAIL;
      goto exit;
    }

    if((context->slot_config[slot].feat & VAULT_ATECC608A_SLOT_FEAT_BUFFER)) {
      break;
    }
  }

  status = atcab_write_bytes_zone(ATCA_ZONE_DATA,
                                  slot,
                                  0,
                                  salt->buffer,
                                  salt->buffer_size);
  if (status != ATCA_SUCCESS) {
    error = OCKAM_VAULT_ERROR_HKDF_SHA256_FAIL;
    goto exit;
  }

  status = atcab_sha_hmac(ikm->buffer,
                          ikm->buffer_size,
                          slot,
                          &tmpkey[0],
                          SHA_MODE_TARGET_TEMPKEY);
  if (status != ATCA_SUCCESS) {
    error = OCKAM_VAULT_ERROR_HKDF_SHA256_FAIL;
    goto exit;
  }

  status = atcab_write_bytes_zone(ATCA_ZONE_DATA,
                                  slot,
                                  0,
                                  &tmpkey[0],
                                  OCKAM_VAULT_HKDF_SHA256_OUTPUT_LENGTH);
  if (status != ATCA_SUCCESS) {
    error = OCKAM_VAULT_ERROR_HKDF_SHA256_FAIL;
    goto exit;
  }

  *prk_slot = slot;

exit:
  return error;
}

/*
 ********************************************************************************************************
 *                                      atecc608a_hkdf_expand()
 ********************************************************************************************************
 */

ockam_error_t atecc608a_hkdf_expand(vault_atecc608a_context_t* context,
                                    ockam_vault_secret_t*      outputs,
                                    uint8_t                    outputs_count,
                                    uint16_t                   prk_slot)
{
  ockam_error_t                     error           = OCKAM_ERROR_NONE;
  uint8_t                           i               = 0;
  uint8_t                           c               = 0;
  vault_atecc608a_secret_context_t* output_ctx      = 0;
  ATCA_STATUS                       status          = ATCA_SUCCESS;
  atca_hmac_sha256_ctx_t            sha_ctx         = {0};
  uint8_t*                          previous_digest = 0;

  if((context == 0) || (outputs == 0) || (outputs_count == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_PARAM;
    goto exit;
  }

  for(i = 1; i <= outputs_count; i++) {
    if(outputs->context != 0) {
      error = OCKAM_VAULT_ERROR_HKDF_SHA256_FAIL;
      goto exit;
    }

    c = i & 0xFF;

    error = ockam_memory_alloc_zeroed(context->memory,
                                      (void**) &(outputs->context),
                                      sizeof(vault_atecc608a_secret_context_t));
    if(error != OCKAM_ERROR_NONE) {
      goto exit;
    }

    output_ctx = (vault_atecc608a_secret_context_t*) outputs->context;

    error = ockam_memory_alloc_zeroed(context->memory,
                                      (void**) &(output_ctx->buffer),
                                      OCKAM_VAULT_HKDF_SHA256_OUTPUT_LENGTH);
    if(error != OCKAM_ERROR_NONE) {
      goto exit;
    }

    output_ctx->buffer_size = OCKAM_VAULT_HKDF_SHA256_OUTPUT_LENGTH;


    error = ockam_memory_set(context->memory,
                             &sha_ctx,
                             0,
                             sizeof(atca_hmac_sha256_ctx_t));
    if(error != OCKAM_ERROR_NONE) {
      goto exit;
    }

    status = atcab_sha_hmac_init(&sha_ctx, prk_slot);
    if(status != ATCA_SUCCESS) {
      error = OCKAM_VAULT_ERROR_HKDF_SHA256_FAIL;
      break;
    }

    if(previous_digest != 0) {
      status = atcab_sha_hmac_update(&sha_ctx,
                                     previous_digest,
                                     OCKAM_VAULT_HKDF_SHA256_OUTPUT_LENGTH);
      if(status != ATCA_SUCCESS) {
        error = OCKAM_VAULT_ERROR_HKDF_SHA256_FAIL;
        break;
      }
    }

    status = atcab_sha_hmac_update(&sha_ctx, &c, 1);
    if(status != ATCA_SUCCESS) {
      error = OCKAM_VAULT_ERROR_HKDF_SHA256_FAIL;
      break;
    }

    status = atcab_sha_hmac_finish(&sha_ctx,
                                   output_ctx->buffer,
                                   SHA_MODE_TARGET_OUT_ONLY);
    if(status != ATCA_SUCCESS) {
      error = OCKAM_VAULT_ERROR_HKDF_SHA256_FAIL;
      break;
    }

    previous_digest = output_ctx->buffer;
    outputs++;
  }

exit:
  return error;
}

/*
 ********************************************************************************************************
 *                                     atecc608a_aead_aes_gcm()
 ********************************************************************************************************
 */

ockam_error_t atecc608a_aead_aes_gcm(ockam_vault_t*        vault,
                                     int                   encrypt,
                                     ockam_vault_secret_t* key,
                                     uint16_t              nonce,
                                     const uint8_t*        additional_data,
                                     size_t                additional_data_length,
                                     const uint8_t*        input,
                                     size_t                input_length,
                                     uint8_t*              output,
                                     size_t                output_size,
                                     size_t*               output_length)
{
  ockam_error_t                     error                                    = OCKAM_ERROR_NONE;
  ockam_error_t                     exit_error                               = OCKAM_ERROR_NONE;
  ATCA_STATUS                       status                                   = ATCA_SUCCESS;
  atca_aes_gcm_ctx_t*               atca_ctx                                 = 0;
  vault_atecc608a_context_t*        context                                  = 0;
  vault_atecc608a_secret_context_t* key_ctx                                  = 0;
  bool                              is_verified                              = false;
  uint32_t                          key_bit_size                             = 0;
  uint8_t                           iv[VAULT_ATECC608A_AEAD_AES_GCM_IV_SIZE] = { 0 };
  uint8_t                           slot                                     = 0;

  if ((vault == 0) || (vault->impl_context == 0)) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  context = (vault_atecc608a_context_t*) vault->impl_context;

  if (encrypt) {
    if (output_size < (input_length + OCKAM_VAULT_AEAD_AES_GCM_TAG_LENGTH)) {
      error = OCKAM_VAULT_ERROR_INVALID_SIZE;
      goto exit;
    }
  }

  if (key->attributes.type != OCKAM_VAULT_SECRET_TYPE_AES128_KEY) {
    error = OCKAM_VAULT_ERROR_INVALID_SECRET_TYPE;
    goto exit;
  }

  if (key->context == 0) {
    error = OCKAM_VAULT_ERROR_INVALID_CONTEXT;
    goto exit;
  }

  key_ctx = (vault_atecc608a_secret_context_t*) key->context;

  for(slot = 0; slot <= VAULT_ATECC608A_NUM_SLOTS; slot++) {
    if(slot == VAULT_ATECC608A_NUM_SLOTS) {
      error = OCKAM_VAULT_ERROR_HKDF_SHA256_FAIL;
      goto exit;
    }

    if((context->slot_config[slot].feat & VAULT_ATECC608A_SLOT_FEAT_AESKEY)) {
      status = atcab_write_bytes_zone(ATCA_ZONE_DATA,
                                      slot,
                                      0,
                                      key_ctx->buffer,
                                      OCKAM_VAULT_AES128_KEY_LENGTH);
      break;
    }
  }

  {
    int n = 1;

    if (*(char*) &n == 1) { /* Check the endianness and copy appropriately */
      iv[VAULT_ATECC608A_AEAD_AES_GCM_IV_OFFSET]     = ((nonce >> 8) & 0xFF);
      iv[VAULT_ATECC608A_AEAD_AES_GCM_IV_OFFSET + 1] = ((nonce) &0xFF);
    } else {
      iv[VAULT_ATECC608A_AEAD_AES_GCM_IV_OFFSET]     = ((nonce) &0xFF);
      iv[VAULT_ATECC608A_AEAD_AES_GCM_IV_OFFSET + 1] = ((nonce >> 8) & 0xFF);
    }
  }

  error = ockam_memory_alloc_zeroed(context->memory,    /* Allocate an AES GCM context struct for either      */
                                    (void **)&atca_ctx, /* encryption or decryption.                          */
                                    sizeof(atca_aes_gcm_ctx_t));
  if (error != OCKAM_ERROR_NONE) {
    goto exit;
  }

  status = atcab_aes_gcm_init(atca_ctx,
                              slot,
                              VAULT_ATECC608A_AES_GCM_KEY_BLOCK,
                              &iv[0],
                              VAULT_ATECC608A_AEAD_AES_GCM_IV_SIZE);
  if (status != ATCA_SUCCESS) {
    error = OCKAM_VAULT_ERROR_AEAD_AES_GCM_FAIL;
    goto exit;
  }

  status = atcab_aes_gcm_aad_update(atca_ctx, /*  Add additional data to GCM                        */
                                    additional_data,
                                    additional_data_length);
  if (status != ATCA_SUCCESS) {
    error = OCKAM_VAULT_ERROR_AEAD_AES_GCM_FAIL;
    goto exit;
  }

  if (encrypt == VAULT_ATECC608A_AEAD_AES_GCM_ENCRYPT) {

    uint8_t* tag_offset = 0;
    status = atcab_aes_gcm_encrypt_update(atca_ctx, /* If mode is encrypt, resulting cipertext is placed  */
                                          input,    /* into output.                                       */
                                          input_length,
                                          output);
    if (status != ATCA_SUCCESS) {
      error = OCKAM_VAULT_ERROR_AEAD_AES_GCM_FAIL;
      goto exit;
    }

    tag_offset = output + input_length;

    status = atcab_aes_gcm_encrypt_finish(atca_ctx,    /* After the cipertext has been generated, output the */
                                          tag_offset,  /* resulting tag to p_tag and end AES GCM encryption  */
                                          OCKAM_VAULT_AEAD_AES_GCM_TAG_LENGTH);
    if (status != ATCA_SUCCESS) {
      error = OCKAM_VAULT_ERROR_AEAD_AES_GCM_FAIL;
      goto exit;
    }

    uint8_t* output_buf = output;
    uint8_t* tag_buf    = tag_offset;

    *output_length = input_length + OCKAM_VAULT_AEAD_AES_GCM_TAG_LENGTH;
  } else if (encrypt == VAULT_ATECC608A_AEAD_AES_GCM_DECRYPT) {
    const uint8_t* tag_offset = 0;
    status = atcab_aes_gcm_decrypt_update(atca_ctx, /* If mode is decrypt, resulting plaintext is placed  */
                                          input,    /* into output.                                       */
                                          (input_length - OCKAM_VAULT_AEAD_AES_GCM_TAG_LENGTH),
                                          output);
    if (status != ATCA_SUCCESS) {
      error = OCKAM_VAULT_ERROR_AEAD_AES_GCM_FAIL;
      goto exit;
    }

    tag_offset = input + (input_length - OCKAM_VAULT_AEAD_AES_GCM_TAG_LENGTH);

    status = atcab_aes_gcm_decrypt_finish(atca_ctx,   /* After the plaintext has been generated, complete   */
                                          tag_offset, /* the GCM decrypt by verifying the auth tag          */
                                          OCKAM_VAULT_AEAD_AES_GCM_TAG_LENGTH,
                                          &is_verified);
    if (status != ATCA_SUCCESS) {
      error = OCKAM_VAULT_ERROR_AEAD_AES_GCM_FAIL;
      goto exit;
    }

    if (!is_verified) {
      error = OCKAM_VAULT_ERROR_AEAD_AES_GCM_FAIL;
      goto exit;
    }

    *output_length = input_length - OCKAM_VAULT_AEAD_AES_GCM_TAG_LENGTH;
  } else {
    error = OCKAM_VAULT_ERROR_AEAD_AES_GCM_FAIL;
    goto exit;
  }

exit:

  if (atca_ctx != 0) {
    ockam_memory_free(context->memory, atca_ctx, sizeof(atca_aes_gcm_ctx_t));
  }

  if(context->mutex) {
    exit_error = ockam_mutex_unlock(context->mutex, context->lock);
    if(error == OCKAM_ERROR_NONE) {
      error = exit_error;
    }
  }

  return error;
}

