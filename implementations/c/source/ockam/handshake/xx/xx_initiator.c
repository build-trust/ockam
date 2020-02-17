/**
 ********************************************************************************************************
 * @file    xx_responder.c
 * @brief   Interface functions for xx handshake responder
 ********************************************************************************************************
 */
#include <string.h>
#include "vault.h"
#include "error.h"
#include "syslog.h"
#include "handshake.h"
#include "handshake_local.h"

extern OCKAM_VAULT_CFG_s vault_cfg;
/*
 ********************************************************************************************************
 *                                           GLOBAL FUNCTIONS                                           *
 ********************************************************************************************************
 */
OCKAM_ERR ockam_xx_initiator_handshake( OCKAM_TRANSPORT_CONNECTION connection, XX_HANDSHAKE* p_h )
{
    OCKAM_ERR                       status = OCKAM_ERR_NONE;
    uint8_t                         send_buffer[MAX_TRANSMIT_SIZE];
    uint8_t                         recv_buffer[MAX_TRANSMIT_SIZE];
    uint16_t                        bytes_received = 0;
    uint16_t                        transmit_size = 0;
    uint8_t                         compare[1024];
    uint32_t                        compare_bytes;

    /* Initialize handshake struct and generate initial static & ephemeral keys */
    status = xx_handshake_prologue( p_h );
    if( OCKAM_ERR_NONE != status ) {
        log_error(status, "Failed handshake prologue");
        goto exit_block;
    }

    // Step 1 generate message
    status = xx_initiator_m1_make( p_h, send_buffer, MAX_TRANSMIT_SIZE, &transmit_size );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "initiator_step_1 failed" );
        goto exit_block;
    }

    // Step 1 send message
    status = ockam_send_blocking( connection, send_buffer, transmit_size );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "ockam_send_blocking after initiator_step_1 failed" );
        goto exit_block;
    }

    // Msg 2 receive
    status = ockam_receive_blocking( connection, recv_buffer, sizeof(recv_buffer), &bytes_received );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "ockam_receive_blocking failed on msg 2" );
        goto exit_block;
    }

    // Msg 2 process
    status = xx_initiator_m2_process( p_h, recv_buffer, bytes_received );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "xx_initiator_m2_process failed on msg 2" );
        goto exit_block;
    }

    // Msg 3 make
    status = xx_initiator_m3_make( p_h, send_buffer, &transmit_size );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "initiator_m3_make failed" );
        goto exit_block;
    }

    // Msg 3 send
    status = ockam_send_blocking( connection, send_buffer, transmit_size );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "ockam_send_blocking failed on msg 3" );
        goto exit_block;
    }

    status = xx_initiator_epilogue( p_h );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "initiator_epilogue failed" );
        goto exit_block;
    }

exit_block:
    return status;
}

/*
 ********************************************************************************************************
 *                                            LOCAL FUNCTIONS                                           *
 ********************************************************************************************************
 */

/*------------------------------------------------------------------------------------------------------*
 *          INITIATOR FUNCTIONS
 *------------------------------------------------------------------------------------------------------*/

OCKAM_ERR xx_initiator_m1_make( XX_HANDSHAKE* p_h,uint8_t* p_send_buffer, uint16_t buffer_length,
                             uint16_t* p_transmit_size )
{
    OCKAM_ERR       status = OCKAM_ERR_NONE;
    uint16_t        offset = 0;

    // Write e to outgoing buffer
    // h = SHA256(h || e.PublicKey
    memcpy( p_send_buffer, p_h->e, KEY_SIZE );
    offset += KEY_SIZE;

    mix_hash( p_h, p_h->e, sizeof(p_h->e) );

    // Write payload to outgoing buffer, payload is empty
    // h = SHA256( h || payload )
    mix_hash( p_h, NULL, 0 );

    *p_transmit_size = offset;

    return status;
}

OCKAM_ERR xx_initiator_m2_process( XX_HANDSHAKE* p_h, uint8_t* p_recv, uint16_t recv_size )
{
    OCKAM_ERR       status = OCKAM_ERR_NONE;
    uint16_t        offset = 0;
    uint8_t         uncipher[MAX_TRANSMIT_SIZE];
    uint8_t         tag[TAG_SIZE];
    uint8_t         vector[VECTOR_SIZE];

    // 1. Read 32 bytes from the incoming
    // message buffer, parse it as a public
    // key, set it to re
    // h = SHA256(h || re)
    memcpy( p_h->re, p_recv, KEY_SIZE );
    offset += KEY_SIZE;
    mix_hash( p_h, p_h->re, KEY_SIZE );

    // 2. ck, k = HKDF(ck, DH(e, re), 2)
    // n = 0
    status = hkdf_dh( p_h->ck, sizeof(p_h->ck), OCKAM_VAULT_KEY_EPHEMERAL, p_h->re, sizeof(p_h->re),
                      KEY_SIZE, p_h->ck, p_h->k );
    if( OCKAM_ERR_NONE != status ) {
        log_error(status, "failed hkdf_dh of prologue in responder_m2_make");
        goto exit_block;
    }
    p_h->nonce = 0;

    // 3. Read 48 bytes of the incoming message buffer as c
    // p = DECRYPT(k, n++, h, c)
    // h = SHA256(h || c),
    // parse p as a public key,
    // set it to rs
    memset( tag, 0, sizeof(tag) );
    memcpy( tag, p_recv+offset+KEY_SIZE, TAG_SIZE );
    make_vector( p_h->nonce, vector );
    status = ockam_vault_aes_gcm_decrypt( p_h->k, KEY_SIZE, vector, sizeof(vector), p_h->h, sizeof(p_h->h),
    tag, sizeof(tag), p_recv+offset, KEY_SIZE, uncipher, KEY_SIZE );
    if( OCKAM_ERR_NONE != status ) {
        log_error(status, "failed ockam_vault_aes_gcm_decrypt in initiator_m2_recv");
        goto exit_block;
    }
    p_h->nonce += 1;
    memcpy( p_h->rs, uncipher, KEY_SIZE );
    mix_hash( p_h, p_recv+offset, KEY_SIZE+TAG_SIZE );
    offset += KEY_SIZE + TAG_SIZE;

    // 4. ck, k = HKDF(ck, DH(e, rs), 2)
    // n = 0
    // secret = ECDH( e, re )
    status = hkdf_dh( p_h->ck,  sizeof(p_h->ck), OCKAM_VAULT_KEY_EPHEMERAL, p_h->rs, sizeof(p_h->rs),
                      KEY_SIZE, p_h->ck, p_h->k );
    if( OCKAM_ERR_NONE != status ) {
        log_error(status, "failed hkdf_dh of prologue in initiator_m2_process");
        goto exit_block;
    }
    p_h->nonce = 0;

    // 5. Read remaining bytes of incoming
    // message buffer as c
    // p = DECRYPT(k, n++, h, c)
    // h = SHA256(h || c),
    // parse p as a payload,
    // payload should be empty
    memset( tag, 0, sizeof(tag) );
    memcpy( tag, p_recv+offset, TAG_SIZE );
    make_vector( p_h->nonce, vector );
    status = ockam_vault_aes_gcm_decrypt( p_h->k, KEY_SIZE, vector, sizeof(vector),
                                          p_h->h, sizeof(p_h->h), tag, sizeof(tag), NULL, 0, NULL, 0 );
    if( OCKAM_ERR_NONE != status ) {
        log_error(status, "failed ockam_vault_aes_gcm_decrypt in initiator_m2_recv");
        goto exit_block;
    }
    p_h->nonce += 1;
    mix_hash( p_h, p_recv+offset, TAG_SIZE );

exit_block:
    return status;
}

OCKAM_ERR xx_initiator_m3_make( XX_HANDSHAKE* p_h, uint8_t* p_msg, uint16_t* p_msg_size )
{

    OCKAM_ERR       status = OCKAM_ERR_NONE;
    uint8_t         tag[TAG_SIZE];
    uint8_t         cipher[KEY_SIZE];
    u_int16_t       offset = 0;
    uint8_t         vector[VECTOR_SIZE];

    // 1. c = ENCRYPT(k, n++, h, s.PublicKey)
    // h =  SHA256(h || c),
    // Write c to outgoing message
    // buffer, BigEndian
    memset( cipher, 0, sizeof(cipher) );
    make_vector( p_h->nonce, vector );
    status = ockam_vault_aes_gcm_encrypt( p_h->k, KEY_SIZE, vector, sizeof(vector), p_h->h, SHA256_SIZE,
                                          tag, TAG_SIZE, p_h->s, KEY_SIZE, cipher, KEY_SIZE );
    if( OCKAM_ERR_NONE != status ) {
        log_error(status, "failed ockam_vault_aes_gcm_encrypt in initiator_m3_make");
        goto exit_block;
    }
    p_h->nonce += 1;
    memcpy( p_msg, cipher, KEY_SIZE );
    offset += KEY_SIZE;
    memcpy( p_msg+offset, tag, TAG_SIZE );
    offset += TAG_SIZE;
    mix_hash( p_h, p_msg, KEY_SIZE+TAG_SIZE );

    // 2. ck, k = HKDF(ck, DH(s, re), 2)
    // n = 0
    status = hkdf_dh( p_h->ck, sizeof(p_h->ck), OCKAM_VAULT_KEY_STATIC, p_h->re, sizeof(p_h->re),
                      KEY_SIZE, p_h->ck, p_h->k );
    if( OCKAM_ERR_NONE != status ) {
        log_error(status, "failed hkdf_dh in initiator_m3_make");
        goto exit_block;
    }
    p_h->nonce = 0;

    // 3. c = ENCRYPT(k, n++, h, payload)
    // h = SHA256(h || c),
    // payload is empty
    make_vector( p_h->nonce, vector );
    status = ockam_vault_aes_gcm_encrypt( p_h->k, KEY_SIZE, vector, sizeof(vector),
                                          p_h->h, sizeof(p_h->h), cipher, TAG_SIZE, NULL, 0, NULL, 0 );

    if( OCKAM_ERR_NONE != status ) {
      log_error(status, "failed hkdf_dh in initiator_m3_make");
      goto exit_block;
    }
    p_h->nonce += 1;
    mix_hash(p_h, cipher, TAG_SIZE);
    memcpy( p_msg+offset, cipher, TAG_SIZE );
    offset += TAG_SIZE;
    // Copy cipher text into send buffer, append tag

    *p_msg_size = offset;

exit_block:
    return status;
}


OCKAM_ERR xx_initiator_epilogue( XX_HANDSHAKE* p_h )
{
    OCKAM_ERR   status		= OCKAM_ERR_NONE;
    uint8_t     keys[2*KEY_SIZE];

    memset(keys, 0, sizeof(keys));
    status = ockam_vault_hkdf( p_h->ck, KEY_SIZE, NULL, 0,  NULL, 0, keys, sizeof(keys));
        if( OCKAM_ERR_NONE != status ) {
        log_error( status, "ockam_vault_hkdf failed in responder_epilogue_make");
    }
    memcpy(p_h->kd, keys, KEY_SIZE );
    memcpy(p_h->ke, &keys[KEY_SIZE], KEY_SIZE );
    p_h->ne = 0;
    p_h->nd = 0;

exit_block:
    return status;
}
