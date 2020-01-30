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

#include <ockam/define.h>
#include <ockam/error.h>

#include <ockam/kal.h>
#include <ockam/memory.h>
#include <ockam/vault.h>
#include <ockam/vault/tpm.h>
#include <ockam/vault/tpm/microchip.h>

#include <cryptoauthlib/lib/cryptoauthlib.h>
#include <cryptoauthlib/lib/atca_cfgs.h>
#include <cryptoauthlib/lib/atca_iface.h>
#include <cryptoauthlib/lib/atca_device.h>
#include <cryptoauthlib/lib/basic/atca_basic_aes_gcm.h>

#if !defined(OCKAM_VAULT_CONFIG_FILE)
#error "Error: Ockam Vault Config File Missing"
#else
#include OCKAM_VAULT_CONFIG_FILE
#endif


/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

#define ATECC608A_DEVREV_MIN                  0x02600000        /* Minimum device rev from info                       */
#define ATECC608A_DEVREV_MAX                  0x026000FF        /* Maximum device rev from info                       */

#define ATECC608A_PMS_SIZE                      32u             /* Size of the pre-master secret                      */
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
 * @struct  ATECC608A_CFG_DATA_s
 * @brief
 *******************************************************************************
 */

#pragma pack(1)
typedef struct {                                                /*!< Byte(s): Description                             */
    uint8_t serial_num_0[4];                                    /*!< 0-3    : SN<0:3>                                 */
    uint32_t revision;                                          /*!< 4-7    : Revision Number                         */
    uint8_t serial_num_1[5];                                    /*!< 8-12   : SN<4:8>                                 */
    uint8_t reserved0;                                          /*!< 13     : Reserved                                */
    uint8_t i2c_enable;                                         /*!< 14     : Bit 0: 0=SingleWire,1=I2C               */
    uint8_t reserved1;                                          /*!< 15     : Reserved                                */
    uint8_t i2c_address;                                        /*!< 16     : I2C Address bits 7-1, bit 0 must be 0   */
    uint8_t reserved2;                                          /*!< 17     : Reserved                                */
    uint8_t otp_mode;                                           /*!< 18     : Configures the OTP zone. RO or RW       */
    uint8_t chip_mode;                                          /*!< 19     : Bit 2-Watchdog,Bit 1-TTL,Bit 0-Selector */
    uint16_t slot_config[16];                                   /*!< 20-51  : 16 slot configurations                  */
    uint8_t counter_0[8];                                       /*!< 52-59  : Counter that can be connected to keys   */
    uint8_t counter_1[8];                                       /*!< 60-67  : Stand-alone counter                     */
    uint8_t last_key_use[16];                                   /*!< 68-83  : Control limited use for KeyID 15        */
    uint8_t user_extra;                                         /*!< 84     : 1 byte value updatedable after data lock*/
    uint8_t selector;                                           /*!< 85     : Selects device to be active after pause */
    uint8_t lock_value;                                         /*!< 86     : Lock state of the Data/OTP zone         */
    uint8_t lock_config;                                        /*!< 87     : Lock state of the configuration zone    */
    uint16_t slot_locked;                                       /*!< 88-89  : Bit for each slot. 0-Locked, 1-Unlocked */
    uint16_t rfu;                                               /*!< 90-91  : Must be 0                               */
    uint32_t x509_format;                                       /*!< 92-95  : Template length & public position config*/
    uint16_t key_config[16];                                    /*!< 96-127 : 16 key configurations                   */
} ATECC608A_CFG_DATA_s;
#pragma pack()


/*
 ********************************************************************************************************
 *                                            INLINE FUNCTIONS                                          *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                            GLOBAL VARIABLES                                          *
 ********************************************************************************************************
 */

static ATECC608A_CFG_DATA_s *g_atecc608a_cfg_data;

static uint8_t g_atecc608a_io_key[] = {                         /* IO Protection Key is used to encrypt data sent via */
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,             /* I2C to the ATECC608A. During init the key is       */
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,             /* written into the device. In a production system    */
    0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27,             /* the key should be locked into the device and never */
    0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37              /* transmitted via I2C.                               */
};


/*
 ********************************************************************************************************
 *                                           GLOBAL FUNCTIONS                                           *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                            LOCAL FUNCTIONS                                           *
 ********************************************************************************************************
 */

OCKAM_ERR atecc608a_write_key(uint8_t *p_key, uint32_t key_size,
                              uint8_t key_slot, uint32_t key_slot_size);

OCKAM_ERR atecc608a_hkdf_extract(uint8_t *p_input, uint32_t input_size,
                                 uint8_t *p_prk, uint32_t prk_size,
                                 uint8_t key_slot);

OCKAM_ERR atecc608a_hkdf_expand(uint8_t key_slot,
                                uint8_t *p_info, uint32_t info_size,
                                uint8_t *p_output, uint32_t output_size);


/*
 ********************************************************************************************************
 ********************************************************************************************************
 *                                         OCKAM_VAULT_CFG_INIT
 ********************************************************************************************************
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_TPM_MICROCHIP_ATECC608A)

/*
 ********************************************************************************************************
 *                                         ockam_vault_tpm_init()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_tpm_init(void *p_arg)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    ATCA_STATUS status = ATCA_SUCCESS;
    VAULT_MICROCHIP_CFG_s *p_atecc608a_cfg;


    do {
        if(p_arg == 0) {                                        /* Ensure the p_arg value is not null                 */
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        p_atecc608a_cfg = (VAULT_MICROCHIP_CFG_s*) p_arg;       /* Grab the vault configuration for the ATECC608A     */

        if(p_atecc608a_cfg->iface == VAULT_MICROCHIP_IFACE_I2C) {
            status = atcab_init(p_atecc608a_cfg->iface_cfg);    /* Call Cryptolib to initialize the ATECC608A via I2C */
            if(status != ATCA_SUCCESS) {
                ret_val = OCKAM_ERR_VAULT_TPM_INIT_FAIL;
                break;
            }
        } else {                                                /* Single-wire or HID is not supported at this time   */
            ret_val = OCKAM_ERR_VAULT_TPM_UNSUPPORTED_IFACE;
            break;
        }
                                                                /* Allocate memory for the configuration structure    */
        ret_val = ockam_mem_alloc((void**) &g_atecc608a_cfg_data,
                                  sizeof(ATECC608A_CFG_DATA_s));
                                                                /* Read the configuration of the ATECC608A            */
        status = atcab_read_config_zone((uint8_t*) g_atecc608a_cfg_data);
        if(status != ATCA_SUCCESS) {
            ret_val = OCKAM_ERR_VAULT_TPM_ID_FAIL;
            break;
        }
                                                                /* Ensure the revision is valid for the ATECC608A     */
        if((g_atecc608a_cfg_data->revision < ATECC608A_DEVREV_MIN) ||
           (g_atecc608a_cfg_data->revision > ATECC608A_DEVREV_MAX)) {
            ret_val = OCKAM_ERR_VAULT_TPM_ID_INVALID;
            break;
        }
                                                                /* Ensure hardware configuration and data is locked   */
        if((g_atecc608a_cfg_data->lock_config != ATECC608A_CFG_LOCK_CONFIG_LOCKED) ||
           (g_atecc608a_cfg_data->lock_value != ATECC608A_CFG_LOCK_CONFIG_LOCKED)) {
            ret_val = OCKAM_ERR_VAULT_TPM_UNLOCKED;
            break;
        }


        ret_val = atecc608a_write_key(&g_atecc608a_io_key[0],   /* Write the IO Protection Key to the specified slot  */
                                      ATECC608A_IO_KEY_SIZE,
                                      ATECC608A_IO_KEY_SLOT,
                                      ATECC608A_IO_KEY_SLOT_SIZE);
        if(ret_val != OCKAM_ERR_NONE) {
            break;
        }
    } while(0);

    return ret_val;
}


/*
 ********************************************************************************************************
 *                                          ockam_vault_tpm_free()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_tpm_free (void)
{
   return OCKAM_ERR_NONE;
}


/*
 ********************************************************************************************************
 *                                        atecc608a_write_key()
 ********************************************************************************************************
 */

OCKAM_ERR atecc608a_write_key(uint8_t *p_key, uint32_t key_size,
                              uint8_t key_slot, uint32_t key_slot_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    ATCA_STATUS status = ATCA_SUCCESS;
    uint8_t i = 0;
    uint8_t slot_offset = 0;
    uint8_t block_offset = 0;
    uint8_t slot_write_4 = 0;
    uint8_t slot_write_32 = 0;
    uint8_t *p_key_buf = 0;
    uint8_t *p_buf = 0;


    do {
        if(key_size > key_slot_size) {                          /* Ensure the key will fit in the specified slot      */
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        ret_val = ockam_mem_alloc((void**)&p_key_buf,           /* Get a buffer for the full size of the key          */
                                  key_slot_size);
        if(ret_val != OCKAM_ERR_NONE) {
            break;
        }

        p_buf = p_key_buf;                                      /* Save the p_key_buf address to free later           */

        do {
            if(key_size > 0) {                                  /* Copy the key into the zero'd buffer, only if there */
                ret_val = ockam_mem_copy(p_buf,                 /* is a valid key. Otherwise, just zero out the key   */
                                         p_key,                 /* slot.                                              */
                                         key_size);
                if(ret_val != OCKAM_ERR_NONE) {
                    break;
                }
            }
                                                                /* Calculate how many 32 and 4 byte reads are needed  */
            slot_write_32 = key_slot_size / ATECC608A_SLOT_WRITE_SIZE_MAX;
            slot_write_4 = (key_slot_size % ATECC608A_SLOT_WRITE_SIZE_MAX) / ATECC608A_SLOT_WRITE_SIZE_MIN;

            slot_offset = 0;                                    /* Always start at the 0 offset for the slot and block*/
            block_offset = 0;

            for(i = 0; i < slot_write_32; i++) {                /* Perform 32 byte writes first. Always increment the */
                status = atcab_write_zone(ATCA_ZONE_DATA,       /* block offset after a 32 byte read but never adjust */
                                          key_slot,             /* the slot offset.                                   */
                                          block_offset,
                                          slot_offset,
                                          p_buf,
                                          ATECC608A_SLOT_WRITE_SIZE_MAX);
                if(status != ATCA_SUCCESS) {
                    break;
                }

                block_offset++;
                p_buf += ATECC608A_SLOT_WRITE_SIZE_MAX;
            }

            if(status != ATCA_SUCCESS) {                        /* Ensure the 32 byte writes were sucessful before    */
                ret_val = OCKAM_ERR_VAULT_TPM_HKDF_FAIL;        /* attempting the 4 byte writes                       */
                break;
            }

            for(i = 0; i < slot_write_4; i++) {                 /* Perform 4 block writes second. Always update the   */
                status = atcab_write_zone(ATCA_ZONE_DATA,       /* slot offset after a write. If slot offset hits 32  */
                                          key_slot,             /* reset and increment block offset.                  */
                                          block_offset,
                                          slot_offset,
                                          p_buf,
                                          ATECC608A_SLOT_WRITE_SIZE_MAX);
                if(status != ATCA_SUCCESS) {
                    break;
                }

                slot_offset++;                                  /* Adjust the offset and buffer pointer AFTER data    */
                p_buf += ATECC608A_SLOT_WRITE_SIZE_MIN;         /* has been sucessfully written to the ATECC608A      */

                if(slot_offset >= ATECC608A_SLOT_OFFSET_MAX) {  /* Always check the slot offset after its been        */
                    slot_offset = 0;                            /* incremented                                        */
                    block_offset++;
                }
            }

            if(status != ATCA_SUCCESS) {                        /* Ensure the 4 byte writes were sucessful before     */
                ret_val = OCKAM_ERR_VAULT_TPM_HKDF_FAIL;        /* proceeding, otherwise unknown data in HKDF slot    */
                break;                                          /* may be used for HKDF                               */
            }
        } while(0);

        ret_val = ockam_mem_free(p_key_buf);                    /* Free the allocated buffer                          */
    } while(0);

    return ret_val;
}


#endif                                                          /* OCKAM_VAULT_CFG_INIT                               */


/*
 ********************************************************************************************************
 ********************************************************************************************************
 *                                         OCKAM_VAULT_CFG_RAND
 ********************************************************************************************************
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_RAND == OCKAM_VAULT_TPM_MICROCHIP_ATECC608A)


/*
 ********************************************************************************************************
 *                                        ockam_vault_tpm_random()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_tpm_random(uint8_t *p_rand_num, uint32_t rand_num_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    ATCA_STATUS status = ATCA_SUCCESS;


    do {
        if(rand_num_size != ATECC608A_RAND_SIZE) {              /* Make sure the expected size matches the buffer     */
            ret_val = OCKAM_ERR_VAULT_SIZE_MISMATCH;
            break;
        }

        status = atcab_random(p_rand_num);                      /* Get a random number from the ATECC608A             */
        if(status != ATCA_SUCCESS) {
            ret_val = OCKAM_ERR_VAULT_TPM_RAND_FAIL;
        }
    } while (0);

    return ret_val;
}

#endif                                                          /* OCKAM_VAULT_CFG_RAND                               */


/*
 ********************************************************************************************************
 ********************************************************************************************************
 *                                      OCKAM_VAULT_CFG_KEY_ECDH
 ********************************************************************************************************
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_KEY_ECDH == OCKAM_VAULT_TPM_MICROCHIP_ATECC608A)


/*
 ********************************************************************************************************
 *                                        ockam_vault_tpm_key_gen()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_tpm_key_gen(OCKAM_VAULT_KEY_e key_type)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    ATCA_STATUS status = ATCA_SUCCESS;
    uint8_t rand[ATECC608A_RAND_SIZE] = {0};


    do
    {
        status = atcab_random(&rand[0]);                        /* Get a random number from the ATECC608A             */
        if(status != ATCA_SUCCESS) {                            /* before a genkey operation.                         */
            ret_val = OCKAM_ERR_VAULT_TPM_KEY_FAIL;
            break;
        }

        status = atcab_nonce((const uint8_t *)&rand[0]);        /* Feed the random number back into the ATECC608A     */
        if(status != ATCA_SUCCESS) {                            /* before a genkey operation.                         */
            ret_val = OCKAM_ERR_VAULT_TPM_KEY_FAIL;
            break;
        }

        if(key_type == OCKAM_VAULT_KEY_STATIC) {                /* Static private key preloaded on ATECC608A          */
            status = atcab_genkey(ATECC608A_KEY_SLOT_STATIC, 0);
            if(status != ATCA_SUCCESS) {
                ret_val = OCKAM_ERR_VAULT_TPM_KEY_FAIL;
                break;
            }
        }

        else if(key_type == OCKAM_VAULT_KEY_EPHEMERAL) {        /* Generate a temp key                                */
            status = atcab_genkey(ATECC608A_KEY_SLOT_EPHEMERAL, 0);
            if(status != ATCA_SUCCESS) {
                ret_val = OCKAM_ERR_VAULT_TPM_KEY_FAIL;
                break;
            }
        }

        else {                                                  /* Invalid parameter, return an error                 */
            ret_val = OCKAM_ERR_INVALID_PARAM;
        }

    } while(0);

    return ret_val;
}


/*
 ********************************************************************************************************
 *                                        ockam_vault_tpm_key_get_pub()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_tpm_key_get_pub(OCKAM_VAULT_KEY_e key_type,
                                      uint8_t *p_pub_key, uint32_t pub_key_size)
{
    ATCA_STATUS status = ATCA_SUCCESS;
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do
    {
        if(p_pub_key == 0) {                                    /* Ensure the buffer isn't null                       */
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        if(pub_key_size != ATECC608A_PUB_KEY_SIZE) {
            ret_val = OCKAM_ERR_VAULT_SIZE_MISMATCH;
            break;
        }

        switch(key_type) {
            case OCKAM_VAULT_KEY_STATIC:                        /* Get the static public key                          */
                status = atcab_get_pubkey(ATECC608A_KEY_SLOT_STATIC,
                                          p_pub_key);

                if(status != ATCA_SUCCESS) {
                    ret_val = OCKAM_ERR_VAULT_TPM_KEY_FAIL;
                }
                break;

            case OCKAM_VAULT_KEY_EPHEMERAL:                     /* Get the generated ephemeral public key             */
                status = atcab_get_pubkey(ATECC608A_KEY_SLOT_EPHEMERAL,
                                          p_pub_key);

                if(status != ATCA_SUCCESS) {
                    ret_val = OCKAM_ERR_VAULT_TPM_KEY_FAIL;
                }
                break;

            default:
                ret_val = OCKAM_ERR_INVALID_PARAM;
                break;
        }
    } while (0);

    return ret_val;
}


/**
 ********************************************************************************************************
 *                                        ockam_vault_tpm_ecdh()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_tpm_ecdh(OCKAM_VAULT_KEY_e key_type,
                               uint8_t *p_pub_key, uint32_t pub_key_size,
                               uint8_t *p_pms, uint32_t pms_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    ATCA_STATUS status = ATCA_SUCCESS;
    uint8_t rand[ATECC608A_RAND_SIZE] = {0};


    do {
        if((p_pub_key == 0) ||                                  /* Ensure the buffers are not null                    */
           (p_pms == 0))
        {
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        if((pub_key_size != ATECC608A_PUB_KEY_SIZE) ||          /* Validate the size of the buffers passed in         */
           (pms_size != ATECC608A_PMS_SIZE))
        {
            ret_val = OCKAM_ERR_VAULT_SIZE_MISMATCH;
            break;
        }

        switch(key_type) {

            case OCKAM_VAULT_KEY_STATIC:                        /* If using the static key, specify which slot        */

                status = atcab_ecdh(ATECC608A_KEY_SLOT_STATIC,
                                        p_pub_key,
                                        p_pms);
                if(status != ATCA_SUCCESS) {
                    ret_val = OCKAM_ERR_VAULT_TPM_ECDH_FAIL;
                }
                break;

            case OCKAM_VAULT_KEY_EPHEMERAL:                     /* If using ephemeral key, specify slot               */

                status = atcab_random(&rand[0]);                /* Get a random number from the ATECC608A             */
                if(status != ATCA_SUCCESS) {                    /* before a genkey operation.                         */
                    ret_val = OCKAM_ERR_VAULT_TPM_KEY_FAIL;
                    break;
                }

                status = atcab_nonce((const uint8_t *)&rand[0]);/* Feed the random number back into the ATECC608A     */
                if(status != ATCA_SUCCESS) {                    /* before a genkey operation.                         */
                    ret_val = OCKAM_ERR_VAULT_TPM_KEY_FAIL;
                    break;
                }

                status = atcab_ecdh(ATECC608A_KEY_SLOT_EPHEMERAL,
                                    p_pub_key,
                                    p_pms);
                if(status != ATCA_SUCCESS) {
                    ret_val = OCKAM_ERR_VAULT_TPM_ECDH_FAIL;
                    break;
                }

                break;

            default:
                ret_val = OCKAM_ERR_INVALID_PARAM;
                break;
        }
    } while (0);

    return ret_val;
}

#endif                                                          /* OCKAM_VAULT_CFG_KEY_ECDH                           */


/*
 ********************************************************************************************************
 ********************************************************************************************************
 *                                        OCKAM_VAULT_CFG_SHA256
 ********************************************************************************************************
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_SHA256 == OCKAM_VAULT_TPM_MICROCHIP_ATECC608A)


/**
 ********************************************************************************************************
 *                                       ockam_vault_tpm_sha256()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_tpm_sha256(uint8_t *p_msg, uint16_t msg_size,
                                 uint8_t *p_digest, uint8_t digest_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    ATCA_STATUS status = ATCA_SUCCESS;


    do {
        status = atcab_sha(msg_size,                            /* Run the SHA256 command in the ATECC608A. The ATCAB */
                           p_msg,                               /* library handles sending data in 32 byte chunks.    */
                           p_digest);
        if(status != ATCA_SUCCESS) {
            ret_val = OCKAM_ERR_VAULT_TPM_SHA256_FAIL;
            break;
        }
    } while(0);

    return ret_val;
}

#endif                                                          /* OCKAM_VAULT_CFG_SHA256                             */


/*
 ********************************************************************************************************
 ********************************************************************************************************
 *                                         OCKAM_VAULT_CFG_HKDF
 ********************************************************************************************************
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_HKDF == OCKAM_VAULT_TPM_MICROCHIP_ATECC608A)

/**
 ********************************************************************************************************
 *                                        ockam_vault_tpm_ecdh()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_tpm_hkdf(uint8_t *p_salt, uint32_t salt_size,
                               uint8_t *p_ikm, uint32_t ikm_size,
                               uint8_t *p_info, uint32_t info_size,
                               uint8_t *p_out, uint32_t out_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    ATCA_STATUS status = ATCA_SUCCESS;
    uint8_t prk[ATECC608A_HMAC_HASH_SIZE];


    do {
        if(salt_size > ATECC608A_HMAC_HASH_SIZE) {              /* Salt must fit in the specified key slot. Depending */
            ret_val = OCKAM_ERR_VAULT_SIZE_MISMATCH;            /* on what slot has been selected, size will vary     */
            break;
        }

        ret_val = atecc608a_write_key(p_salt,                   /* Salt must be written to the key slot before the    */
                                      salt_size,                /* HMAC operation can be performed.                   */
                                      ATECC608A_HKDF_SLOT,
                                      ATECC608A_HKDF_SLOT_SIZE);
        if(ret_val != OCKAM_ERR_NONE) {
            break;
        }

        ret_val = atecc608a_hkdf_extract(p_ikm,                 /* Extract stage of HKDF. Output is the psuedo-random */
                                         ikm_size,              /* key which is used in the expand stage.             */
                                         &prk[0],
                                         ATECC608A_HMAC_HASH_SIZE,
                                         ATECC608A_HKDF_SLOT);
        if(ret_val != OCKAM_ERR_NONE) {
            break;
        }

        ret_val = atecc608a_write_key(&prk[0],                  /* Write the PRK into HKDF key slot for expand stage  */
                                      ATECC608A_HMAC_HASH_SIZE,
                                      ATECC608A_HKDF_SLOT,
                                      ATECC608A_HKDF_SLOT_SIZE);
        if(ret_val != OCKAM_ERR_NONE) {
            break;
        }

        ret_val = atecc608a_hkdf_expand(ATECC608A_HKDF_SLOT,    /* Expand stage of HKDF. Uses the PRK from extract    */
                                        p_info, info_size,      /* and outputs the key at the desired output size.    */
                                        p_out, out_size);
    } while(0);

    return ret_val;
}



/*
 ********************************************************************************************************
 *                                      atecc608a_hkdf_extract()
 ********************************************************************************************************
 */

OCKAM_ERR atecc608a_hkdf_extract(uint8_t *p_input, uint32_t input_size,
                                 uint8_t *p_prk, uint32_t prk_size,
                                 uint8_t key_slot)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    ATCA_STATUS status = ATCA_SUCCESS;


    do {
        if((p_input == 0) != (input_size == 0)) {               /* Ensure input buffer is valid                       */
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        if(prk_size != ATECC608A_HMAC_HASH_SIZE) {              /* PRK buffer must be length of the hash output       */
            ret_val = OCKAM_ERR_VAULT_SIZE_MISMATCH;
        }

        status = atcab_sha_hmac(p_input,                        /* Run HMAC on the input data using the salt located  */
                                input_size,                     /* in the HKDF key slot. Digest is returned to the    */
                                key_slot,                       /* output buffer AND placed in TEMPKEY.               */
                                p_prk,
                                SHA_MODE_TARGET_TEMPKEY);
        if(status != ATCA_SUCCESS)
        {
            ret_val = OCKAM_ERR_VAULT_TPM_HKDF_FAIL;
            break;
        }
    } while (0);

    return ret_val;
}


/*
 ********************************************************************************************************
 *                                      atecc608a_hkdf_expand()
 ********************************************************************************************************
 */

OCKAM_ERR atecc608a_hkdf_expand(uint8_t key_slot,
                                uint8_t *p_info, uint32_t info_size,
                                uint8_t *p_output, uint32_t output_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    ATCA_STATUS status = ATCA_SUCCESS;
    uint8_t i = 0;
    uint8_t iterations = 0;
    uint32_t bytes_written = 0;
    uint32_t bytes_to_copy = 0;
    uint32_t digest_len = 0;
    atca_hmac_sha256_ctx_t *p_ctx = 0;
    uint8_t digest[ATECC608A_HMAC_HASH_SIZE] = {0};


    do {
        if(p_output == 0) {                                     /* Must have a valid output buffer, info is optional  */
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        if((p_info == 0) && (info_size > 0)) {                  /* Info size must be 0 if info pointer is null        */
            ret_val = OCKAM_ERR_INVALID_SIZE;
        }

        iterations  = output_size / ATECC608A_HMAC_HASH_SIZE;   /* Determine how many expand iterations are needed    */
        if(output_size % ATECC608A_HMAC_HASH_SIZE) {
            iterations++;
        }

        if(iterations > 255) {                                  /* RFC 5869 Section 2.3, output size can not be       */
            ret_val = OCKAM_ERR_INVALID_SIZE;                   /* greater than 255 times the hash length             */
            break;
        }

        for(i = 1; i <= iterations; i++) {
            uint8_t c = i & 0xFF;                               /* Set the constant based off the iteration count     */

            ret_val = ockam_mem_alloc((void**)&p_ctx,           /* Allocate HMAC/SHA256 context buffer each iteration */
                                      sizeof(atca_hmac_sha256_ctx_t));
            if(ret_val != OCKAM_ERR_NONE) {
                break;
            }

            status = atcab_sha_hmac_init(p_ctx, key_slot);      /* Initialize HMAC specifying the key slot. The       */
            if(status != ATCA_SUCCESS) {                        /* digest from the extract stage must have already    */
                ret_val = OCKAM_ERR_VAULT_TPM_HKDF_FAIL;        /* been placed into the HKDF key slot BEFORE expand.  */
                break;
            }

            if(digest_len > 0) {                                /* Only add digest buffer after the first iteration   */
                status = atcab_sha_hmac_update(p_ctx,
                                               &digest[0],
                                               digest_len);
                if(status != ATCA_SUCCESS) {
                    ret_val = OCKAM_ERR_VAULT_TPM_HKDF_FAIL;
                    break;
                }
            }

            status = atcab_sha_hmac_update(p_ctx,               /* Add the info context every iteration               */
                                           p_info,
                                           info_size);
            if(status != ATCA_SUCCESS) {
                ret_val = OCKAM_ERR_VAULT_TPM_HKDF_FAIL;
                break;
            }

            status = atcab_sha_hmac_update(p_ctx, &c, 1);       /* Always add the constant last for every iteration   */
            if(status != ATCA_SUCCESS) {
                ret_val = OCKAM_ERR_VAULT_TPM_HKDF_FAIL;
                break;
            }

            status = atcab_sha_hmac_finish(p_ctx,               /* Finish the HMAC calculation. Output the digest to  */
                                           &digest[0],          /* the local buffer and TEMPKEY buffer.               */
                                           SHA_MODE_TARGET_TEMPKEY);
            if(status != ATCA_SUCCESS) {
                ret_val = OCKAM_ERR_VAULT_TPM_HKDF_FAIL;
                break;
            }

            if(i != iterations) {                               /* If there are more iterations, copy the entire      */
                bytes_to_copy = ATECC608A_HMAC_HASH_SIZE;       /* digest to the output                               */
            } else {                                            /* Otherwise, only copy the necessary remaining       */
                bytes_to_copy = output_size - bytes_written;    /* bytes to the output buffer.                        */
            }

            ret_val = ockam_mem_copy((p_output + bytes_written),/* Copy digest data to the output buffer at the       */
                                     &digest[0],                /* specified offset based on past writes.             */
                                     bytes_to_copy);
            if(ret_val != OCKAM_ERR_NONE) {
                break;
            }

            bytes_written += bytes_to_copy;                     /* Update bytes been written for future offsets and   */
            digest_len = bytes_to_copy;                         /* set digest len so its included next iteration      */

            ockam_mem_free(p_ctx);                              /* Free the context buffer after every iteration.     */
            p_ctx = 0;                                          /* Clear the pointer value after freeing to prevent   */
        }                                                       /* a double free.                                     */

        if(p_ctx) {                                             /* If an error occured in the loop and we exited      */
            ockam_mem_free(p_ctx);                              /* early, ensure the buffer is freed.                 */
        }
    } while(0);

    return ret_val;
}

#endif                                                          /* OCKAM_VAULT_CFG_HKDF                               */


/*
 ********************************************************************************************************
 ********************************************************************************************************
 *                                       OCKAM_VAULT_CFG_AES_GCM
 ********************************************************************************************************
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_AES_GCM == OCKAM_VAULT_TPM_MICROCHIP_ATECC608A)


/*
 ********************************************************************************************************
 *                                      ockam_vault_tpm_aes_gcm()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_tpm_aes_gcm(OCKAM_VAULT_AES_GCM_MODE_e mode,
                                  uint8_t *p_key, uint32_t key_size,
                                  uint8_t *p_iv, uint32_t iv_size,
                                  uint8_t *p_aad, uint32_t aad_size,
                                  uint8_t *p_tag, uint32_t tag_size,
                                  uint8_t *p_input, uint32_t input_size,
                                  uint8_t *p_output, uint32_t output_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    OCKAM_ERR t_ret_val = OCKAM_ERR_NONE;
    ATCA_STATUS status = ATCA_SUCCESS;
    atca_aes_gcm_ctx_t *p_ctx = 0;
    bool is_verified = false;
    uint32_t key_bit_size = 0;


    do {
        if((p_key == 0) || (key_size == 0) ||                   /* Key and IV are required for AES GCM. There must be */
           (p_iv == 0) || (iv_size == 0) ||                     /* valid buffers and sizes greater than zero. Tag is  */
           (p_tag == 0) || (tag_size == 0)) {                   /* also always required to be present for encrypt and */
            ret_val = OCKAM_ERR_INVALID_PARAM;                  /* decrypt.                                           */
            break;
        }

        if((p_aad == 0) != (aad_size == 0)) {                   /* Valid for both the AAD buffer and size to be zero  */
            ret_val = OCKAM_ERR_INVALID_PARAM;                  /* or non-zero. Can't have a mismatch.                */
            break;
        }

        if((p_input == 0) != (input_size == 0)) {               /* Input buffer and size must both either be zero or  */
            ret_val = OCKAM_ERR_INVALID_PARAM;                  /* non-zero. Can't have a mismatch.                   */
            break;
        }

        if((p_output == 0) != (output_size == 0)) {             /* Output buffer and size must both either be zero o  */
            ret_val = OCKAM_ERR_INVALID_PARAM;                  /* non-zero. Can't have a mismatch.                   */
            break;
        }

        key_bit_size = key_size * 8;                            /* Key size is specified in bits. Ensure the key      */
        if(key_bit_size != ATECC608A_AES_GCM_KEY_SIZE) {        /* size is set to 128 for the ATECC608A.              */
            ret_val = OCKAM_ERR_VAULT_INVALID_KEY_SIZE;
            break;
        }

        if((p_input == p_output) && (p_input != 0)) {           /* The input buffer can not be used for the result    */
            ret_val = OCKAM_ERR_VAULT_INVALID_BUFFER;
            break;
        }

        if(input_size != output_size) {                         /* Input buffer size must match the output buffer     */
            ret_val = OCKAM_ERR_VAULT_INVALID_BUFFER_SIZE;      /* size, otherwise encrypt/decyrpt fails              */
            break;
        }

        ret_val = atecc608a_write_key(p_key,                    /* Write the AES key to the AES GCM slot              */
                                      key_size,
                                      ATECC608A_AES_GCM_KEY,
                                      ATECC608A_AES_GCM_KEY_SLOT_SIZE);
        if(ret_val != OCKAM_ERR_NONE) {
            break;
        }

        ret_val = ockam_mem_alloc((void**)&p_ctx,               /* Allocate an AES GCM context struct for either      */
                                  sizeof(atca_aes_gcm_ctx_t));  /* encryption or decryption.                          */
        if(ret_val != OCKAM_ERR_NONE) {
            break;
        }

        do {
            status = atcab_aes_gcm_init(p_ctx,                  /* Initialize AES GCM context using the key loaded    */
                                        ATECC608A_AES_GCM_KEY,  /* into TEMPKEY and the supplied IV                   */
                                        ATECC608A_AES_GCM_KEY_BLOCK,
                                        p_iv,
                                        iv_size);
            if(status != ATCA_SUCCESS) {
                ret_val = OCKAM_ERR_VAULT_TPM_AES_GCM_FAIL;
                break;
            }

            status = atcab_aes_gcm_aad_update(p_ctx,            /*  Add additional data to GCM                        */
                                              p_aad,
                                              aad_size);
            if(status != ATCA_SUCCESS) {
                ret_val = OCKAM_ERR_VAULT_TPM_AES_GCM_FAIL;
                break;
            }

            if(mode == OCKAM_VAULT_AES_GCM_MODE_ENCRYPT) {
                status = atcab_aes_gcm_encrypt_update(p_ctx,    /* If mode is encrypt, resulting cipertext is placed  */
                                                      p_input,  /* into p_output.                                     */
                                                      input_size,
                                                      p_output);
                if(status != ATCA_SUCCESS) {
                    ret_val = OCKAM_ERR_VAULT_TPM_AES_GCM_FAIL;
                    break;
                }

                status = atcab_aes_gcm_encrypt_finish(p_ctx,    /* After the cipertext has been generated, output the */
                                                      p_tag,    /* resulting tag to p_tag and end AES GCM encryption  */
                                                      tag_size);
                if(status != ATCA_SUCCESS) {
                    ret_val = OCKAM_ERR_VAULT_TPM_AES_GCM_FAIL;
                    break;
                }
            } else if(mode == OCKAM_VAULT_AES_GCM_MODE_DECRYPT) {
                status = atcab_aes_gcm_decrypt_update(p_ctx,    /* If mode is decrypt, resulting plaintext is placed  */
                                                      p_input,  /* into p_output.                                     */
                                                      input_size,
                                                      p_output);
                if(status != ATCA_SUCCESS) {
                    ret_val = OCKAM_ERR_VAULT_TPM_AES_GCM_FAIL;
                    break;
                }

                status = atcab_aes_gcm_decrypt_finish(p_ctx,    /* After the plaintext has been generated, complete   */
                                                      p_tag,    /* the GCM decrypt by verifying the auth tag          */
                                                      tag_size,
                                                      &is_verified);
                if(status != ATCA_SUCCESS) {
                    ret_val = OCKAM_ERR_VAULT_TPM_AES_GCM_FAIL;
                    break;
                }

                if(!is_verified) {                              /* If auth tag is invalid, return an error            */
                    ret_val = OCKAM_ERR_VAULT_TPM_AES_GCM_DECRYPT_INVALID;
                    break;
                }
            } else {                                            /* Unknown operation, return an error                 */
                ret_val = OCKAM_ERR_INVALID_PARAM;
                break;
            }
        } while(0);

        t_ret_val = ockam_mem_free(p_ctx);                      /* Free the AES GCM context data. If ret_val does not */
        if(ret_val == OCKAM_ERR_NONE) {                         /* contain an error, save the free return code,       */
            ret_val = t_ret_val;                                /* otherwise don't overwrite the existing error       */
        }

    } while(0);

    return ret_val;
}


#endif                                                          /* OCKAM_VAULT_CFG_AES_GCM                            */

