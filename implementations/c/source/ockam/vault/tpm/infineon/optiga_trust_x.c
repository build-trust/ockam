/**
 ********************************************************************************************************
 * @file    optiga_trust_x.c
 * @brief   Ockam Vault Implementation for the Optiga Trust X
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
#include <ockam/vault/tpm/infineon.h>

#include <optiga/optiga_crypt.h>
#include <optiga/optiga_util.h>
#include <optiga/pal/pal.h>
#include <optiga/pal/pal_gpio.h>

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

#define OPTIGA_TRUST_X_RAND_NUM_SIZE_MAX            1024u       /* Data sheet unclear about max size. Limit for now   */

#define OPTIGA_TRUST_X_NUM_KEYS                        2u       /* Only support one static and one ephemeral to start */
#define OPTIGA_TRUST_X_PUB_KEY_SIZE                   64u       /* Keys are NIST P256 with extra data                 */
#define OPTIGA_TRUST_X_PUB_KEY_STATIC                  0u
#define OPTIGA_TRUST_X_PUB_KEY_EPHEMERAL               1u

#define OPTIGA_TRUST_X_PRIV_KEY_SLOT_STATIC     OPTIGA_KEY_STORE_ID_E0F1
#define OPTIGA_TRUST_X_PRIV_KEY_SLOT_EPHEMERAL  OPTIGA_KEY_STORE_ID_E0F2

#define OPTIGA_TRUST_X_SS_SIZE                         32u      /* Shared secret should always be 32 bytes            */

#define OPTIGA_TRUST_X_SHA256_DIGEST_SIZE              32u      /* SHA256 digest always 32 bytes                      */
#define OPTIGA_TRUST_X_SHA256_CTX_BUF_SIZE            130u      /* Context buffer extra space needed for I2C comms    */


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
 * @struct  OPTIGA_TRUST_X_PEER_PUBLIC_KEY_s
 * @brief   Required data structure for receving and sending public keys to
 *          the Optiga Trust X
 *******************************************************************************
 */

#pragma pack(1)
typedef struct {
    uint8_t bit_string_format;                                  /*!< Specifies the format of the string. Always 0x03. */
    uint8_t remaining_length;                                   /*!< Total length excluding this byte and format byte */
    uint8_t reserved_0;                                         /*!< Unused                                           */
    uint8_t compression_format;                                 /*!< Uses 0x04 to specify uncompressed                */
    uint8_t public_key[OPTIGA_TRUST_X_PUB_KEY_SIZE];            /*!< Public key data (64 bytes)                       */
} OPTIGA_TRUST_X_PEER_PUBLIC_KEY_s;
#pragma pack()


/*
 ********************************************************************************************************
 *                                              EXTERNS                                                 *
 ********************************************************************************************************
 */

extern pal_status_t pal_gpio_init(void);
extern pal_status_t pal_os_event_init(void);
extern pal_status_t pal_init(void);


/*
 ********************************************************************************************************
 *                                            GLOBAL VARIABLES                                          *
 ********************************************************************************************************
 */


OPTIGA_TRUST_X_PEER_PUBLIC_KEY_s g_optiga_trust_x_pub_keys[OPTIGA_TRUST_X_NUM_KEYS] = {0};


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

/*
 ********************************************************************************************************
 ********************************************************************************************************
 *                                         OCKAM_VAULT_CFG_INIT
 ********************************************************************************************************
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_INIT & OCKAM_VAULT_TPM_INFINEON_OPTIGA_TRUST_X)

/*
 ********************************************************************************************************
 *                                         ockam_vault_tpm_init()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_tpm_init(void *p_arg)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    pal_status_t pal_status = PAL_STATUS_SUCCESS;
    int32_t status = (int32_t) OPTIGA_LIB_ERROR;
    VAULT_INFINEON_CFG_s *p_optiga_trust_x_cfg = 0;


    do
    {
        if(p_arg == 0) {                                        /* Ensure the p_arg value is not null                 */
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        p_optiga_trust_x_cfg = (VAULT_INFINEON_CFG_s*) p_arg;   /* Grab vault configuration for the Optiga Trust X    */

        pal_status = pal_gpio_init();                           /* GPIO must be inititalized to control reset line    */
        if(pal_status != PAL_STATUS_SUCCESS) {
                ret_val = OCKAM_ERR_VAULT_TPM_INIT_FAIL;
            break;
        }

        pal_status = pal_os_event_init();                       /* OS must be initialized for I2C control             */
        if(pal_status != PAL_STATUS_SUCCESS) {
                ret_val = OCKAM_ERR_VAULT_TPM_INIT_FAIL;
            break;
        }

        pal_status = pal_init();                                /* Finalizes PAL init after GPIO and OS inits         */
        if(pal_status != PAL_STATUS_SUCCESS) {
                ret_val = OCKAM_ERR_VAULT_TPM_INIT_FAIL;
            break;
        }
                                                                /* Open Application is always the first call made     */
        if(p_optiga_trust_x_cfg->iface == VAULT_INFINEON_IFACE_I2C) {
            status = optiga_util_open_application(p_optiga_trust_x_cfg->iface_cfg);
            if(OPTIGA_LIB_SUCCESS != status)
            {
                ret_val = OCKAM_ERR_VAULT_TPM_INIT_FAIL;
                break;
            }
        } else {                                                /* Only supporting I2C at the moment                  */
            ret_val = OCKAM_ERR_VAULT_TPM_UNSUPPORTED_IFACE;
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


#endif                                                          /* OCKAM_VAULT_CFG_INIT                               */


/*
 ********************************************************************************************************
 ********************************************************************************************************
 *                                         OCKAM_VAULT_CFG_RAND
 ********************************************************************************************************
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_RAND == OCKAM_VAULT_TPM_INFINEON_OPTIGA_TRUST_X)


/*
 ********************************************************************************************************
 *                                        ockam_vault_tpm_random()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_tpm_random(uint8_t *p_rand_num, uint32_t rand_num_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    optiga_lib_status_t status = OPTIGA_LIB_SUCCESS;


    do {
        if((rand_num_size <= 0) ||                              /* Make sure the expected size matches the buffer     */
           (rand_num_size > OPTIGA_TRUST_X_RAND_NUM_SIZE_MAX)) {
            ret_val = OCKAM_ERR_VAULT_SIZE_MISMATCH;
            break;
        }

        status = optiga_crypt_random(OPTIGA_RNG_TYPE_TRNG,      /* Generate a random number on the Optiga Trust X. Ok */
                                      p_rand_num,               /* to cast to 16-bit due to size check.               */
                                      (uint16_t) rand_num_size);
        if(status != OPTIGA_LIB_SUCCESS) {
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

#if(OCKAM_VAULT_CFG_KEY_ECDH == OCKAM_VAULT_TPM_INFINEON_OPTIGA_TRUST_X)


/*
 ********************************************************************************************************
 *                                        ockam_vault_tpm_key_gen()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_tpm_key_gen(OCKAM_VAULT_KEY_e key_type)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    optiga_lib_status_t status = OPTIGA_LIB_SUCCESS;
    optiga_key_id_t key_id;
    uint16_t optiga_oid = 0;
    uint16_t offset = 0;
    uint16_t pub_key_len = sizeof(OPTIGA_TRUST_X_PEER_PUBLIC_KEY_s);


    do
    {
        if(key_type == OCKAM_VAULT_KEY_STATIC) {                /* Set the key id based on the specified key type     */
            key_id = OPTIGA_TRUST_X_PRIV_KEY_SLOT_STATIC;
            offset = OPTIGA_TRUST_X_PUB_KEY_STATIC;
        } else if(key_type == OCKAM_VAULT_KEY_EPHEMERAL) {
            key_id = OPTIGA_TRUST_X_PRIV_KEY_SLOT_EPHEMERAL;
            offset = OPTIGA_TRUST_X_PUB_KEY_EPHEMERAL;
        } else {
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }
                                                                /* Generate keypair and do NOT export private key     */
        status = optiga_crypt_ecc_generate_keypair(OPTIGA_ECC_NIST_P_256,
                                                   (OPTIGA_KEY_USAGE_KEY_AGREEMENT | OPTIGA_KEY_USAGE_AUTHENTICATION),
                                                   0,
                                                   &key_id,
                                                   &g_optiga_trust_x_pub_keys[offset],
                                                   &pub_key_len);
        if(status != OPTIGA_LIB_SUCCESS) {
            ret_val = OCKAM_ERR_VAULT_TPM_KEY_FAIL;
            break;
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
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    OPTIGA_TRUST_X_PEER_PUBLIC_KEY_s *p_peer_pub = 0;


    do
    {
        if(p_pub_key == 0) {                                    /* Ensure the buffer isn't null                       */
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        if(pub_key_size != OPTIGA_TRUST_X_PUB_KEY_SIZE) {       /* Ensure the specified public key buffer is the      */
            ret_val = OCKAM_ERR_VAULT_SIZE_MISMATCH;            /* the correct size.                                  */
            break;
        }

        if(key_type == OCKAM_VAULT_KEY_STATIC) {                /* Get the static public key                          */
            p_peer_pub = &g_optiga_trust_x_pub_keys[OPTIGA_TRUST_X_PUB_KEY_STATIC];
        } else if(key_type == OCKAM_VAULT_KEY_EPHEMERAL) {      /* Get the ephemeral public key                       */
            p_peer_pub = &g_optiga_trust_x_pub_keys[OPTIGA_TRUST_X_PUB_KEY_EPHEMERAL];
        } else {
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        if(p_peer_pub->remaining_length == 0) {                 /* Ensure the key has been initialized                */
            ret_val = OCKAM_ERR_VAULT_TPM_KEY_FAIL;
            break;
        }

        ret_val = ockam_mem_copy(p_pub_key,                     /* Extract the public key data from the peer struct   */
                                 p_peer_pub->public_key,        /* to be returned.                                    */
                                 pub_key_size);

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
                               uint8_t *p_ss, uint32_t ss_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    optiga_lib_status_t status = OPTIGA_LIB_SUCCESS;
    public_key_from_host_t optiga_pub_key;
    optiga_key_id_t key_id;
    OPTIGA_TRUST_X_PEER_PUBLIC_KEY_s peer_pub_key;


    do {
        if((p_pub_key == 0) ||                                  /* Ensure the buffers are not null                    */
           (p_ss == 0))
        {
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }

        if((pub_key_size != OPTIGA_TRUST_X_PUB_KEY_SIZE) ||     /* Validate the size of the buffers passed in         */
           (ss_size != OPTIGA_TRUST_X_SS_SIZE))
        {
            ret_val = OCKAM_ERR_VAULT_SIZE_MISMATCH;
            break;
        }

        if(key_type == OCKAM_VAULT_KEY_STATIC) {                /* Set the key id based on the private key to use     */
            key_id = OPTIGA_TRUST_X_PRIV_KEY_SLOT_STATIC;
        } else if(key_type == OCKAM_VAULT_KEY_EPHEMERAL) {
            key_id = OPTIGA_TRUST_X_PRIV_KEY_SLOT_EPHEMERAL;
        } else {
            ret_val = OCKAM_ERR_INVALID_PARAM;
            break;
        }


        peer_pub_key.bit_string_format  = 0x03;                 /* Bit string format                                  */
        peer_pub_key.remaining_length   = 0x42;                 /* 64-byte key + reserved byte and compresstion byte  */
        peer_pub_key.reserved_0         = 0x00;                 /* Unused bits                                        */
        peer_pub_key.compression_format = 0x04;                 /* Compression format - uncompressed                  */

        ockam_mem_copy(&(peer_pub_key.public_key),              /* Copy the received public key into the peer struct  */
                       p_pub_key,                               /* required by the Optiga Trust X                     */
                       pub_key_size);
                                                                /* Configure the public key from host structure for   */
        optiga_pub_key.curve = OPTIGA_ECC_NIST_P_256;           /* the ECDH operation.                                */
        optiga_pub_key.length = sizeof(OPTIGA_TRUST_X_PEER_PUBLIC_KEY_s);
        optiga_pub_key.public_key = (uint8_t*) &peer_pub_key;

        status = optiga_crypt_ecdh(key_id,                      /* Run the ECDH operation on the Optiga Trust X and   */
                                   &optiga_pub_key,             /* place the result in the shared secret buffer.      */
                                   TRUE,
                                   p_ss);
        if(status != OPTIGA_LIB_SUCCESS) {
            ret_val = OCKAM_ERR_VAULT_TPM_KEY_FAIL;
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

#if(OCKAM_VAULT_CFG_SHA256 == OCKAM_VAULT_TPM_INFINEON_OPTIGA_TRUST_X)


/**
 ********************************************************************************************************
 *                                       ockam_vault_tpm_sha256()
 ********************************************************************************************************
 */

OCKAM_ERR ockam_vault_tpm_sha256(uint8_t *p_msg, uint16_t msg_size,
                                 uint8_t *p_digest, uint8_t digest_size)
{
    OCKAM_ERR ret_val = OCKAM_ERR_NONE;
    OCKAM_ERR t_ret_val = OCKAM_ERR_NONE;
    optiga_lib_status_t status = OPTIGA_LIB_SUCCESS;
    optiga_hash_context_t hash_context;
    uint8_t *p_hash_context_buf = 0;
    hash_data_from_host_t hash_data_host;
    uint16_t context_buf_size = OPTIGA_TRUST_X_SHA256_CTX_BUF_SIZE;


    do {
        if(digest_size != OPTIGA_TRUST_X_SHA256_DIGEST_SIZE) {  /* Digest size must always be 32 bytes for SHA256     */
            ret_val = OCKAM_ERR_VAULT_TPM_SHA256_FAIL;
            break;
        }

        do {                                                    /* Allocate a context buffer for the SHA256 operation */
            ret_val = ockam_mem_alloc((void**) &p_hash_context_buf,
                                      context_buf_size);
            if(ret_val != OCKAM_ERR_NONE) {
                break;
            }
                                                                /* Configure the hash context for SHA256              */
            hash_context.hash_algo             = OPTIGA_HASH_TYPE_SHA_256;
            hash_context.context_buffer        = p_hash_context_buf;
            hash_context.context_buffer_length = context_buf_size;

            status = optiga_crypt_hash_start(&hash_context);    /* Pass in the SHA256 context before feeding data     */
            if(status != OPTIGA_LIB_SUCCESS) {
                ret_val = OCKAM_ERR_VAULT_TPM_SHA256_FAIL;
                break;
            }

            hash_data_host.buffer = p_msg;                      /* Set the buffer to the message pointer for the      */
            hash_data_host.length = msg_size;                   /* SHA-256 operation.                                 */

            status = optiga_crypt_hash_update(&hash_context,    /* Run the SHA-256 with the message loaded            */
                                              OPTIGA_CRYPT_HOST_DATA,
                                              &hash_data_host);
            if(status != OPTIGA_LIB_SUCCESS) {
                ret_val = OCKAM_ERR_VAULT_TPM_SHA256_FAIL;
                break;
            }

            status = optiga_crypt_hash_finalize(&hash_context,  /* End the hash context and copy the resuling diegest */
                                                p_digest);      /* into the provided buffer.                          */

            if(status != OPTIGA_LIB_SUCCESS) {
                ret_val = OCKAM_ERR_VAULT_TPM_SHA256_FAIL;
                break;
            }
        } while(0);

        t_ret_val = ockam_mem_free(p_hash_context_buf);         /* Always attempt to free the context buffer          */
        if(ret_val == OCKAM_ERR_NONE) {                         /* Only save the free return status if there are no   */
            ret_val = t_ret_val;                                /* errors up to this point.                           */
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

#if(OCKAM_VAULT_CFG_HKDF == OCKAM_VAULT_TPM_INFINEON_OPTIGA_TRUST_X)
#error "Error: OCKAM_VAULT_CFG_HKDF invalid for INFINEON OPTIGA TRUST X"
#endif                                                          /* OCKAM_VAULT_CFG_HKDF                               */


/*
 ********************************************************************************************************
 ********************************************************************************************************
 *                                       OCKAM_VAULT_CFG_AES_GCM
 ********************************************************************************************************
 ********************************************************************************************************
 */

#if(OCKAM_VAULT_CFG_AES_GCM == OCKAM_VAULT_TPM_INFINEON_OPTIGA_TRUST_X)
#error "Error: OCKAM_VAULT_CFG_AES_GCM invalid for INFINEON OPTIGA TRUST X"
#endif                                                          /* OCKAM_VAULT_CFG_AES_GCM                            */

