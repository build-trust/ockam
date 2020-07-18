/**
 ********************************************************************************************************
 * @file    atecc608a.c
 * @brief   Ockam Vault Implementation for the ATECC608A
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include "ockam/memory.h"
#include "ockam/vault.h"

#include "cryptoauthlib.h"
#include "atca_cfgs.h"
#include "atca_iface.h"
#include "atca_device.h"
#include "basic/atca_basic_aes_gcm.h"

#include "atecc608a.h"

/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

/* clang-format off */

#define ATECC608A_DEVREV_MIN                  0x02600000        /* Minimum device rev from info                       */
#define ATECC608A_DEVREV_MAX                  0x026000FF        /* Maximum device rev from info                       */

#define ATECC608A_SS_SIZE                       32u             /* Size of the shared secret                          */
#define ATECC608A_RAND_SIZE                     32u             /* Size of the random number generated                */
#define ATECC608A_PUB_KEY_SIZE                  64u             /* Size of public key                                 */

#define ATECC608A_SLOT_WRITE_SIZE_MIN            4u             /* Smallest write possible is 4 bytes                 */
#define ATECC608A_SLOT_WRITE_SIZE_MAX           32u             /* Largest write possible is 32 bytes                 */
#define ATECC608A_SLOT_OFFSET_MAX                8u             /* Largest possible offset in slots                   */

#define ATECC608A_KEY_SLOT_STATIC                1u             /* Slot with the preloaded private key                */
#define ATECC608A_KEY_SLOT_EPHEMERAL             2u             /* Slot with the generated ephemeral key              */

#define ATECC608A_CFG_I2C_ENABLE_SHIFT           0u
#define ATECC608A_CFG_I2C_ENABLE_SINGLE_WIRE     0u
#define ATECC608A_CFG_I2C_ENABLE_I2C             1u

#define ATECC608A_CFG_I2C_ADDRESS_SHIFT          1u

#define ATECC608A_CFG_OTP_MODE_READ_ONLY       0xAA             /* Writes to OTP are forbidden                        */
#define ATECC608A_CFG_OTP_MODE_CONSUMPTION     0x55             /* Allows reads and writes to OTP                     */

#define ATECC608A_CFG_CHIP_MODE_WDOG_SHIFT       2u             /* Shift for the watchdog configuration bit           */
#define ATECC608A_CFG_CHIP_MODE_WDOG_1_3_S       0u             /*  Set watchdog to 1.3 seconds - Recommended         */
#define ATECC608A_CFG_CHIP_MODE_WDOG_10_0_S      1u             /*  Set watchdog to 10 seconds                        */

#define ATECC608A_CFG_CHIP_MODE_TTL_SHIFT        1u             /* Shift for TTL Enable                               */
#define ATECC608A_CFG_CHIP_MODE_TTL_FIXED        0u             /*  Input levels use fixed reference                  */
#define ATECC608A_CFG_CHIP_MODE_TTL_VCC          1u             /*  Input levels are VCC referenced                   */

#define ATECC608A_CFG_CHIP_MODE_SEL_SHIFT        0u             /* Shift for Selector Mode                            */
#define ATECC608A_CFG_CHIP_MODE_SEL_ALWAYS       0u             /*  Selector can always be written with UpdateExtra   */
#define ATECC608A_CFG_CHIP_MODE_SEL_LIMITED      1u             /*  Selector can only be written if value is 0        */

#define ATECC608A_CFG_LOCK_VALUE_UNLOCKED      0x55             /* Data and OTP are in an unlocked/configurable state */
#define ATECC608A_CFG_LOCK_VALUE_LOCKED        0x00             /* Data and OTP are in a locked/unconfigurable state  */

#define ATECC608A_CFG_LOCK_CONFIG_UNLOCKED     0x55             /* Config zone is in an unlocked/configurable state   */
#define ATECC608A_CFG_LOCK_CONFIG_LOCKED       0x00             /* Config zone is in a locked/unconfigurable state    */

#define ATECC608A_HKDF_SLOT                      9u             /* Use slot 9 for the HKDF key                        */
#define ATECC608A_HKDF_SLOT_SIZE                72u             /* Slot 9 is 72 bytes                                 */
#define ATECC608A_HKDF_UPDATE_SIZE              64u             /* HMAC updates MUST be 64 bytes                      */
#define ATECC608A_HMAC_HASH_SIZE                32u             /* HMAC hash output size                              */

#define ATECC608A_AES_GCM_KEY                   15u             /* Use slot 15 for the AES Key location               */
#define ATECC608A_AES_GCM_KEY_SIZE             128u             /* ATECC608A only supports AES GCM 128                */
#define ATECC608A_AES_GCM_KEY_BLOCK              0u             /* AES Key starts at block 0 in slot 15               */
#define ATECC608A_AES_GCM_KEY_SLOT_SIZE         72u             /* Size of slot 15 for AES key                        */
#define ATECC608A_AES_GCM_DECRYPT                0u             /* Signal common AES GCM function to decrypt          */
#define ATECC608A_AES_GCM_ENCRYPT                1u             /* Signal common AES GCM function to encrypt          */

#define ATECC608A_IO_KEY_SIZE                   32u             /* IO Protection Key Size                             */
#define ATECC608A_IO_KEY_SLOT                    6u             /* IO Protection Key Slot                             */
#define ATECC608A_IO_KEY_SLOT_SIZE              36u             /* IO Protection Key Slot Size                        */

/*
 ********************************************************************************************************
 *                                               CONSTANTS                                              *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                               DATA TYPES                                             *
 ********************************************************************************************************
 */

/**
 *******************************************************************************
 * @struct  VaultAtecc608aCfg
 * @brief
 *******************************************************************************
 */

#pragma pack(1)
typedef struct {           /*!< Byte(s): Description                             */
  uint8_t SerialNum0[4];   /*!< 0-3    : SN<0:3>                                 */
  uint32_t Revision;       /*!< 4-7    : Revision Number                         */
  uint8_t SerialNum1[5];   /*!< 8-12   : SN<4:8>                                 */
  uint8_t Reserved0;       /*!< 13     : Reserved                                */
  uint8_t I2CEnable;       /*!< 14     : Bit 0: 0=SingleWire,1=I2C               */
  uint8_t Reserved1;       /*!< 15     : Reserved                                */
  uint8_t I2CAddress;      /*!< 16     : I2C Address bits 7-1, bit 0 must be 0   */
  uint8_t Reserved2;       /*!< 17     : Reserved                                */
  uint8_t OtpMode;         /*!< 18     : Configures the OTP zone. RO or RW       */
  uint8_t ChipMode;        /*!< 19     : Bit 2-Watchdog,Bit 1-TTL,Bit 0-Selector */
  uint16_t SlotConfig[16]; /*!< 20-51  : 16 slot configurations                  */
  uint8_t Counter0[8];     /*!< 52-59  : Counter that can be connected to keys   */
  uint8_t Counter1[8];     /*!< 60-67  : Stand-alone counter                     */
  uint8_t LastKeyUse[16];  /*!< 68-83  : Control limited use for KeyID 15        */
  uint8_t UserExtra;       /*!< 84     : 1 byte value updatedable after data lock*/
  uint8_t Selector;        /*!< 85     : Selects device to be active after pause */
  uint8_t LockValue;       /*!< 86     : Lock state of the Data/OTP zone         */
  uint8_t LockConfig;      /*!< 87     : Lock state of the configuration zone    */
  uint16_t SlotLocked;     /*!< 88-89  : Bit for each slot. 0-Locked, 1-Unlocked */
  uint16_t Rfu;            /*!< 90-91  : Must be 0                               */
  uint32_t X509Format;     /*!< 92-95  : Template length & public position config*/
  uint16_t KeyConfig[16];  /*!< 96-127 : 16 key configurations                   */
} VaultAtecc608aCfg;
#pragma pack()

/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

OckamError VaultAtecc608aCreate(OckamVaultCtx **ctx, OckamVaultAtecc608aConfig *p_cfg,
                                const OckamMemory *memory);

OckamError VaultAtecc608aDestroy(OckamVaultCtx *p_ctx);

OckamError VaultAtecc608aRandom(OckamVaultCtx *p_ctx,
                                uint8_t *p_num, size_t num_size);

OckamError VaultAtecc608aKeyGenerate(OckamVaultCtx *p_ctx, OckamVaultKey key_type);

OckamError VaultAtecc608aKeySetPrivate(OckamVaultCtx *p_ctx, OckamVaultKey key_type,
                                       uint8_t *p_priv_key, size_t priv_key_size);

OckamError VaultAtecc608aKeyGetPublic(OckamVaultCtx *p_ctx, OckamVaultKey key_type,
                                      uint8_t *p_pub_key, size_t pub_key_size);

OckamError VaultAtecc608aEcdh(OckamVaultCtx *p_ctx, OckamVaultKey key_type,
                              uint8_t *p_pub_key, size_t pub_key_size,
                              uint8_t *p_ss, size_t ss_size);

OckamError VaultAtecc608aSha256(OckamVaultCtx *p_ctx,
                                uint8_t *p_msg, size_t msg_size,
                                uint8_t *p_digest, size_t digest_size);

OckamError VaultAtecc608aHkdf(OckamVaultCtx *p_ctx,
                              uint8_t *p_salt, size_t salt_size,
                              uint8_t *p_ikm, size_t ikm_size,
                              uint8_t *p_info, size_t info_size,
                              uint8_t *p_out, size_t out_size);

OckamError VaultAtecc608aAesGcmEncrypt(OckamVaultCtx *p_ctx,
                                       uint8_t *p_key, size_t key_size,
                                       uint8_t *p_iv, size_t iv_size,
                                       uint8_t *p_aad, size_t aad_size,
                                       uint8_t *p_tag, size_t tag_size,
                                       uint8_t *p_input, size_t input_size,
                                       uint8_t *p_output, size_t output_size);

OckamError VaultAtecc608aAesGcmDecrypt(OckamVaultCtx *p_ctx,
                                       uint8_t *p_key, size_t key_size,
                                       uint8_t *p_iv, size_t iv_size,
                                       uint8_t *p_aad, size_t aad_size,
                                       uint8_t *p_tag, size_t tag_size,
                                       uint8_t *p_input, size_t input_size,
                                       uint8_t *p_output, size_t output_size);

OckamError Atecc608aWriteKey(const OckamMemory *p_memory,
                             uint8_t *p_key, uint32_t key_size,
                             uint8_t key_slot, uint32_t key_slot_size);

OckamError Atecc608aHkdfExtract(uint8_t *p_input, size_t input_size,
                                uint8_t *p_prk, size_t prk_size,
                                uint8_t key_slot);

OckamError Atecc608aHkdfExpand(const OckamMemory *p_memory, uint8_t key_slot,
                               uint8_t *p_info, size_t info_size,
                               uint8_t *p_output, size_t output_size);

OckamError Atecc608aAesGcm(OckamVaultCtx *p_ctx, int encrypt,
                           uint8_t *p_key, size_t key_size,
                           uint8_t *p_iv, size_t iv_size,
                           uint8_t *p_aad, size_t aad_size,
                           uint8_t *p_tag, size_t tag_size,
                           uint8_t *p_input, size_t input_size,
                           uint8_t *p_output, size_t output_size);

/*
 ********************************************************************************************************
 *                                            GLOBAL VARIABLES                                          *
 ********************************************************************************************************
 */

const OckamVault ockam_vault_atecc608a = {
    (OckamError(*)(void **, void *,
                   const OckamMemory *))   &VaultAtecc608aCreate,

    (OckamError(*)(void *))                &VaultAtecc608aDestroy,

    (OckamError(*)(void *,
                   uint8_t *, size_t))     &VaultAtecc608aRandom,

    (OckamError(*)(void *, OckamVaultKey)) &VaultAtecc608aKeyGenerate,

    (OckamError(*)(void *, OckamVaultKey,
                   uint8_t *, size_t))     &VaultAtecc608aKeyGetPublic,

    (OckamError(*)(void *, OckamVaultKey,
                   uint8_t *, size_t))     &VaultAtecc608aKeySetPrivate,

    (OckamError(*)(void *, OckamVaultKey,
                   uint8_t *, size_t,
                   uint8_t *, size_t))     &VaultAtecc608aEcdh,

    (OckamError(*)(void *,
                   uint8_t *, size_t,
                   uint8_t *, size_t))     &VaultAtecc608aSha256,

    (OckamError(*)(void *,
                   uint8_t *, size_t,
                   uint8_t *, size_t,
                   uint8_t *, size_t,
                   uint8_t *, size_t))     &VaultAtecc608aHkdf,

    (OckamError(*)(void *,
                   uint8_t *, size_t,
                   uint8_t *, size_t,
                   uint8_t *, size_t,
                   uint8_t *, size_t,
                   uint8_t *, size_t,
                   uint8_t *, size_t))     &VaultAtecc608aAesGcmEncrypt,

    (OckamError(*)(void *,
                   uint8_t *, size_t,
                   uint8_t *, size_t,
                   uint8_t *, size_t,
                   uint8_t *, size_t,
                   uint8_t *, size_t,
                   uint8_t *, size_t))     &VaultAtecc608aAesGcmDecrypt,
};

static VaultAtecc608aCfg g_atecc608a_cfg_data = {0};

static uint8_t g_atecc608a_io_key[] = {
                                                    /* IO Protection Key is used to encrypt data sent via */
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, /* I2C to the ATECC608A. During init the key is       */
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, /* written into the device. In a production system    */
    0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, /* the key should be locked into the device and never */
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37  /* transmitted via I2C.                               */
};

/* clang-format on */

/*
 ********************************************************************************************************
 *                                           MODULE FUNCTIONS                                           *
 ********************************************************************************************************
 */

/**
 ********************************************************************************************************
 *                                         VaultAtecc608aCreate()
 ********************************************************************************************************
 */

OckamError VaultAtecc608aCreate(OckamVaultCtx **ctx, OckamVaultAtecc608aConfig *p_cfg, const OckamMemory *memory) {
  OckamError ret_val = kOckamErrorNone;
  OckamVaultCtx *p_ctx = 0;
  ATCA_STATUS status = ATCA_SUCCESS;

  if ((p_cfg == 0) || (memory == 0) || (ctx == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  status = atcab_init(p_cfg->p_atca_iface_cfg);
  if (status != ATCA_SUCCESS) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if (!(g_atecc608a_cfg_data.Revision)) {
    status = atcab_read_config_zone((uint8_t *)&g_atecc608a_cfg_data);
    if (status != ATCA_SUCCESS) {
      ret_val = kOckamError;
      goto exit_block;
    }
  }

  if ((g_atecc608a_cfg_data.Revision < ATECC608A_DEVREV_MIN) ||
      (g_atecc608a_cfg_data.Revision > ATECC608A_DEVREV_MAX)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if ((g_atecc608a_cfg_data.LockConfig != ATECC608A_CFG_LOCK_CONFIG_LOCKED) ||
      (g_atecc608a_cfg_data.LockValue != ATECC608A_CFG_LOCK_CONFIG_LOCKED)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  ret_val = Atecc608aWriteKey(memory, &g_atecc608a_io_key[0], ATECC608A_IO_KEY_SIZE, ATECC608A_IO_KEY_SLOT,
                              ATECC608A_IO_KEY_SLOT_SIZE);
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

  ret_val = memory->Alloc((void **)ctx,           /* Allocate a context structure for this vault        */
                          sizeof(OckamVaultCtx)); /* Ensure a context structure was allocated,          */
  if (ret_val != kOckamErrorNone) {               /* otherwise return 0 to signal failure.              */
    goto exit_block;
  }

  p_ctx = *ctx;
  p_ctx->memory = memory;

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                      VaultAtecc608aDestroy()
 ********************************************************************************************************
 */

OckamError VaultAtecc608aDestroy(OckamVaultCtx *p_ctx) {
  OckamError ret_val = kOckamErrorNone;
  const OckamMemory *p_memory = 0;

  if ((p_ctx == 0) || (p_ctx->memory == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  p_memory = p_ctx->memory;
  p_memory->Free(p_ctx, sizeof(OckamVaultCtx));

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                       VaultAtecc608aRandom()
 ********************************************************************************************************
 */

OckamError VaultAtecc608aRandom(OckamVaultCtx *p_ctx, uint8_t *p_num, size_t num_size) {
  OckamError ret_val = kOckamErrorNone;
  ATCA_STATUS status = ATCA_SUCCESS;

  if (num_size != ATECC608A_RAND_SIZE) {
    ret_val = kOckamError;
    goto exit_block;
  }

  status = atcab_random(p_num);
  if (status != ATCA_SUCCESS) {
    ret_val = kOckamError;
    goto exit_block;
  }

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                     VaultAtecc608aKeyGenerate()
 ********************************************************************************************************
 */

OckamError VaultAtecc608aKeyGenerate(OckamVaultCtx *p_ctx, OckamVaultKey key_type) {
  OckamError ret_val = kOckamErrorNone;
  ATCA_STATUS status = ATCA_SUCCESS;
  uint8_t rand[ATECC608A_RAND_SIZE] = {0};

  status = atcab_random(&rand[0]); /* Get a random number from the ATECC608A             */
  if (status != ATCA_SUCCESS) {    /* before a genkey operation.                         */
    ret_val = kOckamError;
    goto exit_block;
  }

  status = atcab_nonce((const uint8_t *)&rand[0]); /* Feed the random number back into the ATECC608A     */
  if (status != ATCA_SUCCESS) {                    /* before a genkey operation.                         */
    ret_val = kOckamError;
    goto exit_block;
  }

  if (key_type == kOckamVaultKeyStatic) {
    status = atcab_genkey(ATECC608A_KEY_SLOT_STATIC, 0);
    if (status != ATCA_SUCCESS) {
      ret_val = kOckamError;
      goto exit_block;
    }
  }

  else if (key_type == kOckamVaultKeyEphemeral) {
    status = atcab_genkey(ATECC608A_KEY_SLOT_EPHEMERAL, 0);
    if (status != ATCA_SUCCESS) {
      ret_val = kOckamError;
      goto exit_block;
    }
  }

  else {
    ret_val = kOckamError;
  }

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                  VaultAtecc608aKeySetPrivate()
 ********************************************************************************************************
 */

OckamError VaultAtecc608aKeySetPrivate(OckamVaultCtx *p_ctx, OckamVaultKey key_type, uint8_t *p_priv_key,
                                       size_t priv_key_size) {
  OckamError ret_val = kOckamError;

  return ret_val;
}

/**
 ********************************************************************************************************
 *                                  VaultAtecc608aKeyGetPublic()
 ********************************************************************************************************
 */

OckamError VaultAtecc608aKeyGetPublic(OckamVaultCtx *p_ctx, OckamVaultKey key_type, uint8_t *p_pub_key,
                                      size_t pub_key_size) {
  ATCA_STATUS status = ATCA_SUCCESS;
  OckamError ret_val = kOckamErrorNone;

  if (p_pub_key == 0) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if (pub_key_size != ATECC608A_PUB_KEY_SIZE) {
    ret_val = kOckamError;
    goto exit_block;
  }

  switch (key_type) {
    case kOckamVaultKeyStatic:
      status = atcab_get_pubkey(ATECC608A_KEY_SLOT_STATIC, p_pub_key);

      if (status != ATCA_SUCCESS) {
        ret_val = kOckamError;
      }
      goto exit_block;

    case kOckamVaultKeyEphemeral:
      status = atcab_get_pubkey(ATECC608A_KEY_SLOT_EPHEMERAL, p_pub_key);
      if (status != ATCA_SUCCESS) {
        ret_val = kOckamError;
      }
      goto exit_block;

    default:
      ret_val = kOckamError;
      goto exit_block;
      goto exit_block;
  }

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                       VaultAtecc608aEcdh()
 ********************************************************************************************************
 */

OckamError VaultAtecc608aEcdh(OckamVaultCtx *p_ctx, OckamVaultKey key_type, uint8_t *p_pub_key, size_t pub_key_size,
                              uint8_t *p_ss, size_t ss_size) {
  OckamError ret_val = kOckamErrorNone;
  ATCA_STATUS status = ATCA_SUCCESS;
  uint8_t rand[ATECC608A_RAND_SIZE] = {0};

  if ((p_pub_key == 0) || (p_ss == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if ((pub_key_size != ATECC608A_PUB_KEY_SIZE) || (ss_size != ATECC608A_SS_SIZE)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  status = atcab_random(&rand[0]);
  if (status != ATCA_SUCCESS) {
    ret_val = kOckamError;
    goto exit_block;
  }

  status = atcab_nonce((const uint8_t *)&rand[0]);
  if (status != ATCA_SUCCESS) {
    ret_val = kOckamError;
    goto exit_block;
  }

  switch (key_type) {
    case kOckamVaultKeyStatic:

      status = atcab_ecdh(ATECC608A_KEY_SLOT_STATIC, p_pub_key, p_ss);
      if (status != ATCA_SUCCESS) {
        ret_val = kOckamError;
      }
      goto exit_block;

    case kOckamVaultKeyEphemeral:

      status = atcab_ecdh(ATECC608A_KEY_SLOT_EPHEMERAL, p_pub_key, p_ss);
      if (status != ATCA_SUCCESS) {
        ret_val = kOckamError;
      }
      goto exit_block;

    default:
      ret_val = kOckamError;
      goto exit_block;
      goto exit_block;
  }

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                       VaultAtecc608aSha256()
 ********************************************************************************************************
 */

OckamError VaultAtecc608aSha256(OckamVaultCtx *p_ctx, uint8_t *p_msg, size_t msg_size, uint8_t *p_digest,
                                size_t digest_size) {
  OckamError ret_val = kOckamErrorNone;
  ATCA_STATUS status = ATCA_SUCCESS;

  status = atcab_sha(msg_size, p_msg, p_digest);
  if (status != ATCA_SUCCESS) {
    ret_val = kOckamError;
    goto exit_block;
  }

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                       VaultAtecc608aHkdf()
 ********************************************************************************************************
 */

OckamError VaultAtecc608aHkdf(OckamVaultCtx *p_ctx, uint8_t *p_salt, size_t salt_size, uint8_t *p_ikm, size_t ikm_size,
                              uint8_t *p_info, size_t info_size, uint8_t *p_out, size_t out_size) {
  OckamError ret_val = kOckamErrorNone;
  ATCA_STATUS status = ATCA_SUCCESS;
  uint8_t prk[ATECC608A_HMAC_HASH_SIZE] = {0};

  if ((p_ctx == 0) || (p_ctx->memory == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if (salt_size > ATECC608A_HKDF_SLOT_SIZE) { /* Salt value must be able to fit in the HKDF key     */
    ret_val = kOckamError;                    /* slot, which can vary based on what slot is chosen  */
    goto exit_block;
  }

  ret_val = Atecc608aWriteKey(p_ctx->memory, /* Salt must be written to the key slot before the    */
                              p_salt,        /* HMAC operation can be performed.                   */
                              salt_size, ATECC608A_HKDF_SLOT, ATECC608A_HKDF_SLOT_SIZE);
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

  ret_val = Atecc608aHkdfExtract(p_ikm,    /* Extract stage of HKDF. Output is the psuedo-random */
                                 ikm_size, /* key which is used in the expand stage.             */
                                 &prk[0], ATECC608A_HMAC_HASH_SIZE, ATECC608A_HKDF_SLOT);
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

  ret_val = Atecc608aWriteKey(p_ctx->memory, /* Write the PRK into HKDF key slot for expand stage  */
                              &prk[0], ATECC608A_HMAC_HASH_SIZE, ATECC608A_HKDF_SLOT, ATECC608A_HKDF_SLOT_SIZE);
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

  ret_val = Atecc608aHkdfExpand(p_ctx->memory,       /* Expand stage of HKDF. Uses the PRK from extract    */
                                ATECC608A_HKDF_SLOT, /* and outputs the key at the desired output size.    */
                                p_info, info_size, p_out, out_size);

exit_block:
  return ret_val;
}

/**
 ********************************************************************************************************
 *                                  VaultAtecc608aAesGcmDecrypt()
 ********************************************************************************************************
 */

VaultError VaultAtecc608aAesGcmEncrypt(OckamVaultCtx *p_ctx, uint8_t *p_key, size_t key_size, uint8_t *p_iv,
                                       size_t iv_size, uint8_t *p_aad, size_t aad_size, uint8_t *p_tag, size_t tag_size,
                                       uint8_t *p_input, size_t input_size, uint8_t *p_output, size_t output_size) {
  return Atecc608aAesGcm(p_ctx, ATECC608A_AES_GCM_ENCRYPT, p_key, key_size, p_iv, iv_size, p_aad, aad_size, p_tag,
                         tag_size, p_input, input_size, p_output, output_size);
}

/**
 ********************************************************************************************************
 *                                  VaultAtecc608aAesGcmDecrypt()
 ********************************************************************************************************
 */

VaultError VaultAtecc608aAesGcmDecrypt(OckamVaultCtx *p_ctx, uint8_t *p_key, size_t key_size, uint8_t *p_iv,
                                       size_t iv_size, uint8_t *p_aad, size_t aad_size, uint8_t *p_tag, size_t tag_size,
                                       uint8_t *p_input, size_t input_size, uint8_t *p_output, size_t output_size) {
  return Atecc608aAesGcm(p_ctx, ATECC608A_AES_GCM_DECRYPT, p_key, key_size, p_iv, iv_size, p_aad, aad_size, p_tag,
                         tag_size, p_input, input_size, p_output, output_size);
}

/*
 ********************************************************************************************************
 *                                            LOCAL FUNCTIONS                                           *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                      Atecc608aHkdfExtract()
 ********************************************************************************************************
 */

OckamError Atecc608aHkdfExtract(uint8_t *p_input, size_t input_size, uint8_t *p_prk, size_t prk_size,
                                uint8_t key_slot) {
  OckamError ret_val = kOckamErrorNone;
  ATCA_STATUS status = ATCA_SUCCESS;

  if ((p_input == 0) != (input_size == 0)) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if (prk_size != ATECC608A_HMAC_HASH_SIZE) { /* PRK buffer must be length of the hash output       */
    ret_val = kOckamError;
    goto exit_block;
  }

  status = atcab_sha_hmac(p_input,    /* Run HMAC on the input data using the salt located  */
                          input_size, /* in the HKDF key slot. Digest is returned to the    */
                          key_slot,   /* output buffer AND placed in TEMPKEY.               */
                          p_prk, SHA_MODE_TARGET_TEMPKEY);
  if (status != ATCA_SUCCESS) {
    ret_val = kOckamError;
    goto exit_block;
  }

exit_block:
  return ret_val;
}

/*
 ********************************************************************************************************
 *                                      Atecc608aHkdfExpand()
 ********************************************************************************************************
 */

OckamError Atecc608aHkdfExpand(const OckamMemory *p_memory, uint8_t key_slot, uint8_t *p_info, size_t info_size,
                               uint8_t *p_output, size_t output_size) {
  OckamError ret_val = kOckamErrorNone;
  ATCA_STATUS status = ATCA_SUCCESS;

  uint8_t i = 0;
  uint8_t iterations = 0;
  uint32_t bytes_written = 0;
  uint32_t bytes_to_copy = 0;
  uint32_t digest_len = 0;
  uint8_t digest[ATECC608A_HMAC_HASH_SIZE] = {0};
  atca_hmac_sha256_ctx_t *p_atca_ctx = 0;

  if (p_output == 0) { /* Must have a valid output buffer, info is optional  */
    ret_val = kOckamError;
    goto exit_block;
  }

  if ((p_info == 0) && (info_size > 0)) { /* Info size must be 0 if info pointer is null        */
    ret_val = kOckamError;
    goto exit_block;
  }

  iterations = output_size / ATECC608A_HMAC_HASH_SIZE; /* Determine how many expand iterations are needed    */
  if (output_size % ATECC608A_HMAC_HASH_SIZE) {
    iterations++;
  }

  if (iterations > 255) {  /* RFC 5869 Section 2.3, output size can not be       */
    ret_val = kOckamError; /* greater than 255 times the hash length             */
    goto exit_block;
  }

  for (i = 1; i <= iterations; i++) { /* Set the constant based off the iteration count     */
    uint8_t c = i & 0xFF;

    ret_val = p_memory->Alloc((void **)&p_atca_ctx, /* Allocate HMAC/SHA256 context buffer each iteration */
                              sizeof(atca_hmac_sha256_ctx_t));
    if (ret_val != kOckamErrorNone) {
      goto exit_block;
    }

    status = atcab_sha_hmac_init(p_atca_ctx, key_slot); /* Initialize HMAC specifying the key slot. The       */
    if (status != ATCA_SUCCESS) {                       /* digest from the extract stage must have already    */
      ret_val = kOckamError;                            /* been placed into the HKDF key slot BEFORE expand.  */
      goto exit_block;
    }

    if (digest_len > 0) { /* Only add digest buffer after the first iteration   */
      status = atcab_sha_hmac_update(p_atca_ctx, &digest[0], digest_len);
      if (status != ATCA_SUCCESS) {
        ret_val = kOckamError;
        goto exit_block;
      }
    }

    status = atcab_sha_hmac_update(p_atca_ctx, /* Add the info context every iteration               */
                                   p_info, info_size);
    if (status != ATCA_SUCCESS) {
      ret_val = kOckamError;
      goto exit_block;
    }

    status = atcab_sha_hmac_update(p_atca_ctx, &c, 1); /* Always add the constant last for every iteration   */
    if (status != ATCA_SUCCESS) {
      ret_val = kOckamError;
      goto exit_block;
    }

    status = atcab_sha_hmac_finish(p_atca_ctx, /* Finish the HMAC calculation. Output the digest to  */
                                   &digest[0], /* the local buffer and TEMPKEY buffer.               */
                                   SHA_MODE_TARGET_TEMPKEY);
    if (status != ATCA_SUCCESS) {
      ret_val = kOckamError;
      goto exit_block;
    }

    if (i != iterations) {                         /* If there are more iterations, copy the entire      */
      bytes_to_copy = ATECC608A_HMAC_HASH_SIZE;    /* digest to the output                               */
    } else {                                       /* Otherwise, only copy the necessary remaining       */
      bytes_to_copy = output_size - bytes_written; /* bytes to the output buffer.                        */
    }

    ret_val = p_memory->Copy((p_output + bytes_written), /* Copy digest data to the output buffer at the       */
                             &digest[0],                 /* specified offset based on past writes.             */
                             bytes_to_copy);
    if (ret_val != kOckamErrorNone) {
      goto exit_block;
    }

    bytes_written += bytes_to_copy; /* Update bytes been written for future offsets and   */
    digest_len = bytes_to_copy;     /* set digest len so its included next iteration      */

    p_memory->Free(p_atca_ctx, /* Free the context buffer after every iteration.     */
                   sizeof(atca_hmac_sha256_ctx_t));

    p_atca_ctx = 0; /* Clear the pointer value after freeing to prevent   */
  }                 /* a double free.                                     */

exit_block:

  if (p_atca_ctx) {            /* If an error occured in the loop and we exited      */
    p_memory->Free(p_atca_ctx, /* early, ensure the buffer is freed.                 */
                   sizeof(atca_hmac_sha256_ctx_t));
  }

  return ret_val;
}

/*
 ********************************************************************************************************
 *                                          Atecc608aWriteKey()
 ********************************************************************************************************
 */

OckamError Atecc608aWriteKey(const OckamMemory *p_memory, uint8_t *p_key, uint32_t key_size, uint8_t key_slot,
                             uint32_t key_slot_size) {
  OckamError ret_val = kOckamErrorNone;
  ATCA_STATUS status = ATCA_SUCCESS;
  uint8_t i = 0;
  uint8_t slot_offset = 0;
  uint8_t block_offset = 0;
  uint8_t slot_write_4 = 0;
  uint8_t slot_write_32 = 0;
  uint8_t *p_key_buf = 0;
  uint8_t *p_buf = 0;

  if (p_memory == 0) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if (key_size > key_slot_size) {
    ret_val = kOckamError;
    goto exit_block;
  }

  ret_val = p_memory->Alloc((void **)&p_key_buf, key_slot_size);
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

  p_buf = p_key_buf;

  if (key_size > 0) {                                 /* Copy the key into the zero'd buffer, only if there      */
    ret_val = p_memory->Copy(p_buf, p_key, key_size); /* is a valid key. Otherwise, just zero out the key slot.  */
    if (ret_val != kOckamErrorNone) {
      goto exit_block;
    }
  }

  slot_write_32 = key_slot_size / ATECC608A_SLOT_WRITE_SIZE_MAX;
  slot_write_4 = (key_slot_size % ATECC608A_SLOT_WRITE_SIZE_MAX) / ATECC608A_SLOT_WRITE_SIZE_MIN;

  slot_offset = 0;
  block_offset = 0;

  for (i = 0; i < slot_write_32; i++) {
    status =
        atcab_write_zone(ATCA_ZONE_DATA, key_slot, block_offset, slot_offset, p_buf, ATECC608A_SLOT_WRITE_SIZE_MAX);
    if (status != ATCA_SUCCESS) {
      goto exit_block;
    }

    block_offset++;
    p_buf += ATECC608A_SLOT_WRITE_SIZE_MAX;
  }

  if (status != ATCA_SUCCESS) { /* Ensure the 32 byte writes were sucessful before    */
    ret_val = kOckamError;      /* attempting the 4 byte writes                       */
    goto exit_block;
  }

  for (i = 0; i < slot_write_4; i++) {
    status =
        atcab_write_zone(ATCA_ZONE_DATA, key_slot, block_offset, slot_offset, p_buf, ATECC608A_SLOT_WRITE_SIZE_MAX);
    if (status != ATCA_SUCCESS) {
      goto exit_block;
    }

    slot_offset++;                          /* Adjust the offset and buffer pointer AFTER data    */
    p_buf += ATECC608A_SLOT_WRITE_SIZE_MIN; /* has been sucessfully written to the ATECC608A      */

    if (slot_offset >= ATECC608A_SLOT_OFFSET_MAX) { /* Always check the slot offset after its been        */
      slot_offset = 0;                              /* incremented                                        */
      block_offset++;
    }
  }

  if (status != ATCA_SUCCESS) { /* Ensure the 4 byte writes were sucessful before     */
    ret_val = kOckamError;      /* proceeding, otherwise unknown data in HKDF slot    */
    goto exit_block;            /* may be used for HKDF                               */
  }

exit_block:
  if (p_key_buf != 0) {
    p_memory->Free(p_key_buf, key_slot_size);
  }

  return ret_val;
}

/*
 ********************************************************************************************************
 *                                          Atecc608aAesGcm()
 ********************************************************************************************************
 */

OckamError Atecc608aAesGcm(OckamVaultCtx *p_ctx, int encrypt, uint8_t *p_key, size_t key_size, uint8_t *p_iv,
                           size_t iv_size, uint8_t *p_aad, size_t aad_size, uint8_t *p_tag, size_t tag_size,
                           uint8_t *p_input, size_t input_size, uint8_t *p_output, size_t output_size) {
  OckamError ret_val = kOckamErrorNone;
  ATCA_STATUS status = ATCA_SUCCESS;
  atca_aes_gcm_ctx_t *p_atca_ctx = 0;
  bool is_verified = false;
  uint32_t key_bit_size = 0;

  if ((p_ctx == 0) || (p_ctx->memory == 0)) { /* Valid context pointer and memory object required   */
    ret_val = kOckamError;
    goto exit_block;
  }

  if ((p_key == 0) || (key_size == 0) || /* Key and IV are required for AES GCM. There must be */
      (p_iv == 0) || (iv_size == 0) ||   /* valid buffers and sizes greater than zero. Tag is  */
      (p_tag == 0) || (tag_size == 0)) { /* also always required to be present for encrypt and */
    ret_val = kOckamError;               /* decrypt.                                           */
    goto exit_block;
  }

  if ((p_aad == 0) != (aad_size == 0)) { /* Valid for both the AAD buffer and size to be zero  */
    ret_val = kOckamError;               /* or non-zero. Can't have a mismatch.                */
    goto exit_block;
  }

  if ((p_input == 0) != (input_size == 0)) { /* Input buffer and size must both either be zero or  */
    ret_val = kOckamError;                   /* non-zero. Can't have a mismatch.                   */
    goto exit_block;
  }

  if ((p_output == 0) != (output_size == 0)) { /* Output buffer and size must both either be zero o  */
    ret_val = kOckamError;                     /* non-zero. Can't have a mismatch.                   */
    goto exit_block;
  }

  key_bit_size = key_size * 8;                      /* Key size is specified in bits. Ensure the key      */
  if (key_bit_size != ATECC608A_AES_GCM_KEY_SIZE) { /* size is set to 128 for the ATECC608A.              */
    ret_val = kOckamError;
    goto exit_block;
  }

  if ((p_input == p_output) && (p_input != 0)) { /* The input buffer can not be used for the result    */
    ret_val = kOckamError;
    goto exit_block;
  }

  if (input_size != output_size) { /* Input buffer size must match the output buffer     */
    ret_val = kOckamError;         /* size, otherwise encrypt/decyrpt fails              */
    goto exit_block;
  }

  ret_val = Atecc608aWriteKey(p_ctx->memory, /* Write the AES key to the AES GCM slot              */
                              p_key, key_size, ATECC608A_AES_GCM_KEY, ATECC608A_AES_GCM_KEY_SLOT_SIZE);
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

  ret_val = p_ctx->memory->Alloc((void **)&p_atca_ctx,        /* Allocate an AES GCM context struct for either      */
                                 sizeof(atca_aes_gcm_ctx_t)); /* encryption or decryption.                          */
  if (ret_val != kOckamErrorNone) {
    goto exit_block;
  }

  status = atcab_aes_gcm_init(p_atca_ctx,            /* Initialize AES GCM context using the key loaded    */
                              ATECC608A_AES_GCM_KEY, /* into TEMPKEY and the supplied IV                   */
                              ATECC608A_AES_GCM_KEY_BLOCK, p_iv, iv_size);
  if (status != ATCA_SUCCESS) {
    ret_val = kOckamError;
    goto exit_block;
  }

  status = atcab_aes_gcm_aad_update(p_atca_ctx, /*  Add additional data to GCM                        */
                                    p_aad, aad_size);
  if (status != ATCA_SUCCESS) {
    ret_val = kOckamError;
    goto exit_block;
  }

  if (encrypt == ATECC608A_AES_GCM_ENCRYPT) {
    status = atcab_aes_gcm_encrypt_update(p_atca_ctx, /* If mode is encrypt, resulting cipertext is placed  */
                                          p_input,    /* into p_output.                                     */
                                          input_size, p_output);
    if (status != ATCA_SUCCESS) {
      ret_val = kOckamError;
      goto exit_block;
    }

    status = atcab_aes_gcm_encrypt_finish(p_atca_ctx, /* After the cipertext has been generated, output the */
                                          p_tag,      /* resulting tag to p_tag and end AES GCM encryption  */
                                          tag_size);
    if (status != ATCA_SUCCESS) {
      ret_val = kOckamError;
      goto exit_block;
    }
  } else if (encrypt == ATECC608A_AES_GCM_DECRYPT) {
    status = atcab_aes_gcm_decrypt_update(p_atca_ctx, /* If mode is decrypt, resulting plaintext is placed  */
                                          p_input,    /* into p_output.                                     */
                                          input_size, p_output);
    if (status != ATCA_SUCCESS) {
      ret_val = kOckamError;
      goto exit_block;
    }

    status = atcab_aes_gcm_decrypt_finish(p_atca_ctx, /* After the plaintext has been generated, complete   */
                                          p_tag,      /* the GCM decrypt by verifying the auth tag          */
                                          tag_size, &is_verified);
    if (status != ATCA_SUCCESS) {
      ret_val = kOckamError;
      goto exit_block;
    }

    if (!is_verified) {
      ret_val = kOckamError;
      goto exit_block;
    }
  } else {
    ret_val = kOckamError;
    goto exit_block;
  }

exit_block:

  if (p_atca_ctx != 0) {
    p_ctx->memory->Free(p_atca_ctx, sizeof(atca_aes_gcm_ctx_t));
  }

  return ret_val;
}
