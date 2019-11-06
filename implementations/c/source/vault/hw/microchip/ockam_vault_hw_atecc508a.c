/**
 ********************************************************************************************************
 * @file        ockam_vault_hw_atecc508a.c
 * @author      Mark Mulrooney <mark@ockam.io>
 * @copyright   Copyright (c) 2019, Ockam Inc.
 * @brief   
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include <ockam_def.h>
#include <ockam_err.h>

#include <kal/ockam_kal.h>
#include <mem/ockam_mem.h>
#include <vault/ockam_vault.h>
#include <vault/ockam_vault_hw_microchip.h>

#include <cryptoauthlib/lib/cryptoauthlib.h>
#include <cryptoauthlib/lib/atca_cfgs.h>
#include <cryptoauthlib/lib/atca_iface.h>
#include <cryptoauthlib/lib/atca_device.h>

#include <ockam_vault_cfg.h>


/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

#define VAULT_MICROCHIP_ATECC508A_DEVREV_MIN        0x00005000  /* Minimum device rev from info                         */
#define VAULT_MICROCHIP_ATECC508A_DEVREV_MAX        0x000050FF  /* Maximum device rev from info                         */

#define VAULT_ATECC508A_PMS_SIZE                    32u         /* Size of the pre-master secret                        */
#define VAULT_ATECC508A_RAND_SIZE                   32u         /* Size of the random number generated                  */
#define VAULT_ATECC508A_PUB_KEY_SIZE                64u         /* Size of public key                                   */

#define VAULT_ATECC508A_KEY_SLOT_STATIC              0u         /* Slot with the preloaded private key                  */
#define VAULT_ATECC508A_KEY_SLOT_EPHEMERAL   ATCA_TEMPKEY_KEYID /* Slot with the generated ephemeral key                */


#define VAULT_ATECC508A_CFG_I2C_ENABLE_SHIFT        0u
#define VAULT_ATECC508A_CFG_I2C_ENABLE_SINGLE_WIRE  0u
#define VAULT_ATECC508A_CFG_I2C_ENABLE_I2C          1u

#define VAULT_ATECC508A_CFG_I2C_ADDRESS_SHIFT       1u

#define VAULT_ATECC508A_CFG_OTP_MODE_READ_ONLY      0xAA        /* Writes to OTP are forbidden                          */
#define VAULT_ATECC508A_CFG_OTP_MODE_CONSUMPTION    0x55        /* Allows reads and writes to OTP                       */

#define VAULT_ATECC508A_CFG_CHIP_MODE_WDOG_SHIFT    2u          /* Shift for the watchdog configuration bit             */
#define VAULT_ATECC508A_CFG_CHIP_MODE_WDOG_1_3_S    0u          /*  Set watchdog to 1.3 seconds - Recommended           */
#define VAULT_ATECC508A_CFG_CHIP_MODE_WDOG_10_0_S   1u          /*  Set watchdog to 10 seconds                          */

#define VAULT_ATECC508A_CFG_CHIP_MODE_TTL_SHIFT     1u          /* Shift for TTL Enable                                 */
#define VAULT_ATECC508A_CFG_CHIP_MODE_TTL_FIXED     0u          /*  Input levels use fixed reference                    */
#define VAULT_ATECC508A_CFG_CHIP_MODE_TTL_VCC       1u          /*  Input levels are VCC referenced                     */

#define VAULT_ATECC508A_CFG_CHIP_MODE_SEL_SHIFT     0u          /* Shift for Selector Mode                              */
#define VAULT_ATECC508A_CFG_CHIP_MODE_SEL_ALWAYS    0u          /*  Selector can always be written with UpdateExtra     */
#define VAULT_ATECC508A_CFG_CHIP_MODE_SEL_LIMITED   1u          /*  Selector can only be written if value is 0          */

#define VAULT_ATECC508A_CFG_LOCK_VALUE_UNLOCKED     0x55        /* Data and OTP are in an unlocked/configurable state   */
#define VAULT_ATECC508A_CFG_LOCK_VALUE_LOCKED       0x00        /* Data and OTP are in a locked/unconfigurable state    */

#define VAULT_ATECC508A_CFG_LOCK_CONFIG_UNLOCKED    0x55        /* Config zone is in an unlocked/configurable state     */
#define VAULT_ATECC508A_CFG_LOCK_CONFIG_LOCKED      0x00        /* Config zone is in a locked/unconfigurable state      */


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
 * @struct  VAULT_ATECC508A_CFG_DATA_s
 * @brief   
 *******************************************************************************
 */
typedef struct {                                                /*!< Byte(s): Description                               */
    uint8_t serial_num_0[4];                                    /*!< 0-3    : SN<0:3>                                   */
    uint32_t revision;                                          /*!< 4-7    : Revision Number                           */
    uint8_t serial_num_1[5];                                    /*!< 8-12   : SN<4:8>                                   */
    uint8_t reserved0;                                          /*!< 13     : Reserved                                  */
    uint8_t i2c_enable;                                         /*!< 14     : Bit 0: 0=SingleWire,1=I2C                 */
    uint8_t reserved1;                                          /*!< 15     : Reserved                                  */
    uint8_t i2c_address;                                        /*!< 16     : I2C Address bits 7-1, bit 0 must be 0     */
    uint8_t reserved2;                                          /*!< 17     : Reserved                                  */
    uint8_t otp_mode;                                           /*!< 18     : Configures the OTP zone. RO or RW         */
    uint8_t chip_mode;                                          /*!< 19     : Bit 2-Watchdog,Bit 1-TTL,Bit 0-Selector   */
    uint16_t slot_config[16];                                   /*!< 20-51  : 16 slot configurations                    */
    uint8_t counter_0[8];                                       /*!< 52-59  : Counter that can be connected to keys     */
    uint8_t counter_1[8];                                       /*!< 60-67  : Stand-alone counter                       */
    uint8_t last_key_use[16];                                   /*!< 68-83  : Control limited use for KeyID 15          */
    uint8_t user_extra;                                         /*!< 84     : 1 byte value updatedable after data lock  */
    uint8_t selector;                                           /*!< 85     : Selects device to be active after pause   */
    uint8_t lock_value;                                         /*!< 86     : Lock state of the Data/OTP zone           */
    uint8_t lock_config;                                        /*!< 87     : Lock state of the configuration zone      */
    uint16_t slot_locked;                                       /*!< 88-89  : Bit for each slot. 0-Locked, 1-Unlocked   */
    uint16_t rfu;                                               /*!< 90-91  : Must be 0                                 */
    uint32_t x509_format;                                       /*!< 92-95  : Template length & public position config  */
    uint16_t key_config[16];                                    /*!< 96-127 : 16 key configurations                     */
} VAULT_ATECC508A_CFG_DATA_s;


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

static VAULT_ATECC508A_CFG_DATA_s *g_atecc508a_cfg_data;


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


/**
 ********************************************************************************************************
 *                                         ockam_vault_hw_init()
 *
 * @brief   Initialize the ATECC508A for Ockam Vault
 *
 * @param   p_arg   Optional void* argument
 * 
 * @return  OCKAM_ERR_NONE if initialized successfully. OCKAM_ERR_VAULT_ALREADY_INIT if already
 *          initialized. Other errors if specific chip fails init.
 *
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_HW_MICROCHIP_ATECC508A)
OCKAM_ERR ockam_vault_hw_init(void *p_arg)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    ATCA_STATUS status;
    VAULT_MICROCHIP_CFG_s *atecc508a_cfg;


    do {
        if(p_arg == 0) {                                        /* Ensure the p_arg value is not null                   */
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        atecc508a_cfg = (VAULT_MICROCHIP_CFG_s*) p_arg;         /* Grab the vault configuration for the ATECC508A       */

        if(atecc508a_cfg->iface == VAULT_MICROCHIP_IFACE_I2C) {
            status = atcab_init(atecc508a_cfg->iface_cfg);      /* Call Cryptolib to initialize the ATECC508A via I2C   */
            if(status != ATCA_SUCCESS) {
                ret_val = OCKAM_ERR_VAULT_HW_INIT_FAIL;
                break;
            }
        } else {                                                /* Single-wire or HID is not supported at this time     */
            ret_val = OCKAM_ERR_VAULT_HW_UNSUPPORTED_IFACE;
            break;
        }

        ret_val = ockam_mem_alloc((void*) g_atecc508a_cfg_data, /* Allocate memory for the configuration structure      */
                                  sizeof(VAULT_ATECC508A_CFG_DATA_s));
                                                                /* Read the configuration of the ATECC508A              */
        status = atcab_read_config_zone((uint8_t*) g_atecc508a_cfg_data);
        if(status != ATCA_SUCCESS) {
            ret_val = OCKAM_ERR_VAULT_HW_ID_FAIL;
            break;
        }
                                                                /* Ensure the revision is valid for the ATECC508A       */
        if((g_atecc508a_cfg_data->revision < VAULT_MICROCHIP_ATECC508A_DEVREV_MIN) ||
           (g_atecc508a_cfg_data->revision > VAULT_MICROCHIP_ATECC508A_DEVREV_MAX)) {
            ret_val = OCKAM_ERR_VAULT_HW_ID_INVALID;
            break;
        }
                                                                /* Ensure the hardware configuration and data is locked */
        if((g_atecc508a_cfg_data->lock_config != VAULT_ATECC508A_CFG_LOCK_CONFIG_LOCKED) ||
           (g_atecc508a_cfg_data->lock_value != VAULT_ATECC508A_CFG_LOCK_CONFIG_LOCKED)) {
            ret_val = OCKAM_ERR_VAULT_HW_UNLOCKED;
            break;
        }
    } while(0);

    return ret_val;
}
#endif


/**
 ********************************************************************************************************
 *                                          ockam_vault_hw_free()
 *
 * @brief   Free the hardware and all associated data structures
 *
 * @return  OCKAM_ERR_NONE on success.
 * 
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_HW_MICROCHIP_ATECC508A)
OCKAM_ERR ockam_vault_hw_free (void)
{
   return OCKAM_ERR_NONE; 
}
#endif


/**
 ********************************************************************************************************
 *                                        ockam_vault_hw_random()
 *
 * @brief   Generate and return a random number
 *
 * @param   p_rand_num[out]     32-byte array to be filled with the random number
 *
 * @param   rand_num_size[in]   The size of the desired random number & buffer passed in. Used to verify
 *                              correct size.
 * 
 * @return  OCKAM_ERR_NONE if successful. OCKAM_ERR_VAULT_INVALID_SIZE if size 
 *
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_HW_MICROCHIP_ATECC508A)
OCKAM_ERR ockam_vault_hw_random(uint8_t *p_rand_num, uint32_t rand_num_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do {
        if(rand_num_size != VAULT_ATECC508A_RAND_SIZE) {        /* Make sure the expected size matches the buffer       */
            ret_val = OCKAM_ERR_VAULT_SIZE_MISMATCH;
            break;
        }

        atcab_random(p_rand_num);                               /* Get a random number from the ATECC508A               */
    } while (0);

    return ret_val;
}
#endif


/**
 ********************************************************************************************************
 *                                        ockam_vault_hw_key_gen()
 *
 * @brief   Generate an keypair on the ATECC508A
 *
 * @param   key_type[in]    The type of key pair to generate.
 *
 * @return  OCKAM_ERR_NONE if successful.
 *
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_HW_MICROCHIP_ATECC508A)
OCKAM_ERR ockam_vault_hw_key_gen(OCKAM_VAULT_KEY_e key_type)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do
    {
        if(key_type == OCKAM_VAULT_KEY_STATIC) {                /* Static private key preloaded on ATECC508A            */
            atcab_genkey(VAULT_ATECC508A_KEY_SLOT_STATIC, 0);
        }

        else if(key_type == OCKAM_VAULT_KEY_EPHEMERAL) {        /* Generate a temp key                                  */
            atcab_genkey(VAULT_ATECC508A_KEY_SLOT_EPHEMERAL, 0);
            atcab_genkey(ATCA_TEMPKEY_KEYID, 0);
        }

        else {                                                  /* Invalid parameter, return an error                   */
            ret_val = OCKAM_ERR_INVALID_PARAM;
        }

    } while(0);

    return ret_val;
}
#endif


/**
 ********************************************************************************************************
 *                                        ockam_vault_hw_key_get_pub()
 *
 * @brief   Get a public key from the ATECC508A
 *
 * @param   key_type[in]        OCKAM_VAULT_KEY_STATIC if requesting static public key
 *                              OCKAM_VAULT_KEY_EPHEMERAL if requesting the ephemeral public key
 *
 * @param   p_pub_key[out]      Buffer to place the public key in
 *
 * @param   pub_key_size[in]    Size of the public key buffer
 *
 * @return  OCKAM_ERR_NONE if successful.
 *
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_HW_MICROCHIP_ATECC508A)
OCKAM_ERR ockam_vault_hw_key_get_pub(OCKAM_VAULT_KEY_e key_type,
                                     uint8_t *p_pub_key,
                                     uint32_t pub_key_size)
{
    ATCA_STATUS status;
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;


    do
    {
        if(p_pub_key == OCKAM_NULL) {                           /* Ensure the buffer isn't null */
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        if(pub_key_size != VAULT_ATECC508A_PUB_KEY_SIZE) {
            ret_val = OCKAM_ERR_VAULT_SIZE_MISMATCH;
            break;
        }

        switch(key_type) {
            case OCKAM_VAULT_KEY_STATIC:                        /* Get the static public key                            */
                status = atcab_genkey(VAULT_ATECC508A_KEY_SLOT_STATIC,
                                      p_pub_key);

                if(status != ATCA_SUCCESS) {
                    ret_val = OCKAM_ERR_VAULT_HW_KEY_FAIL;
                }
                break;

            case OCKAM_VAULT_KEY_EPHEMERAL:                     /* Get the generated ephemeral public key               */
                status = atcab_genkey(VAULT_ATECC508A_KEY_SLOT_EPHEMERAL,
                                       p_pub_key);

                if(status != ATCA_SUCCESS) {
                    ret_val = OCKAM_ERR_VAULT_HW_KEY_FAIL;
                }
                break;

            default:
                ret_val = OCKAM_ERR_INVALID_PARAM;
                break;
        }
    } while (0);

    return ret_val;
}
#endif



/**
 ********************************************************************************************************
 *                                        ockam_vault_hw_ecdh()
 *
 * @brief   Perform ECDH using the specified key
 *
 * @param   key_type[in]        Specify which key type to use in the ECDH execution
 *
 * @param   p_pub_key[in]       Buffer with the public key
 *
 * @param   pub_key_size[in]    Size of the public key buffer
 *
 * @param   p_pms[out]          Pre-master secret from ECDH
 *
 * @param   pms_size[in]        Size of the pre-master secret buffer
 *
 * @return  OCKAM_ERR_NONE if successful.
 *
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_HW_MICROCHIP_ATECC508A)
OCKAM_ERR ockam_vault_hw_ecdh(OCKAM_VAULT_KEY_e key_type,
                              uint8_t *p_pub_key,
                              uint32_t pub_key_size,
                              uint8_t *p_pms,
                              uint32_t pms_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    ATCA_STATUS status;


    do {
        if((p_pub_key == 0) ||                                  /* Ensure the buffers are not null                      */
           (p_pms == 0))
        {
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        if((pub_key_size != VAULT_ATECC508A_PUB_KEY_SIZE) ||    /* Validate the size of the buffers passed in           */
           (pms_size != VAULT_ATECC508A_PMS_SIZE))
        {
            ret_val = OCKAM_ERR_VAULT_SIZE_MISMATCH;
            break;
        }

        switch(key_type) {

            case OCKAM_VAULT_KEY_STATIC:                        /* If using the static key, specify which slot          */

                status = atcab_ecdh(VAULT_ATECC508A_KEY_SLOT_STATIC,
                                    p_pub_key,
                                    p_pms);
                if(status != ATCA_SUCCESS) {
                    ret_val = OCKAM_ERR_VAULT_HW_ECDH_FAIL;
                }
                break;

            case OCKAM_VAULT_KEY_EPHEMERAL:                     /* Ephemeral key uses the temp key slot on the ATECC508A*/

                status = atcab_ecdh_tempkey(p_pub_key,
                                            p_pms);
                if(status != ATCA_SUCCESS) {
                    ret_val = OCKAM_ERR_VAULT_HW_ECDH_FAIL;
                }
                break;

            default:
                ret_val = OCKAM_ERR_INVALID_PARAM;
                break;
        }
    } while (0);

    return ret_val;
}
#endif

