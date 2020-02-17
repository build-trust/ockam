/**
 ********************************************************************************************************
 * @file    handshake_responder_test.c
 * @brief   Test program for the xx handshake as per Noise XX 25519 AESGCM SHA256
 ********************************************************************************************************
 */
 #include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include "ockam/syslog.h"
#include "ockam/error.h"
#include "ockam/vault.h"
#include "ockam/transport.h"
#include "ockam/handshake.h"
#include "handshake_local.h"
#include "handshake_test.h"


//!! revisit, vault configuration will ultimately go somewhere else
OCKAM_VAULT_CFG_s vault_cfg =
    {
        .p_tpm                       = 0,
        .p_host                      = 0,
        OCKAM_VAULT_EC_CURVE25519
    };

/**
 ********************************************************************************************************
 *                                          test_responder_prologue()
 ********************************************************************************************************
 *
 * Summary: This differs from the production handshake_prologue in that it initiates the handshake
 *          with a known set of keys so that cipher results can be verified along the way.
 *
 * @param p_h [in/out] - pointer to handshake struct
 * @return [out] - OCKAM_ERR_NONE on success
 ********************************************************************************************************
 */
OCKAM_ERR test_responder_prologue( XX_HANDSHAKE* p_h )
{
    OCKAM_ERR       status = OCKAM_ERR_NONE;
    uint8_t         key[KEY_SIZE];
    uint32_t        key_bytes;

    // 1. Pick a static 25519 keypair for this handshake and set it to s
    string_to_hex( RESPONDER_STATIC, key, &key_bytes );
    status = ockam_vault_key_write( OCKAM_VAULT_KEY_STATIC, key, KEY_SIZE );
    if( OCKAM_ERR_NONE != status ) {
        log_error(status, "failed to generate static keypair in initiator_step_1");
        goto exit_block;
    }

    status = ockam_vault_key_get_pub( OCKAM_VAULT_KEY_STATIC, p_h->s, KEY_SIZE );
    if( OCKAM_ERR_NONE != status ) {
        log_error(status, "failed to generate get static public key in initiator_step_1");
        goto exit_block;
    }

    // 2. Generate an ephemeral 25519 keypair for this handshake and set it to e
    string_to_hex( RESPONDER_EPH, key, &key_bytes );
    status = ockam_vault_key_write( OCKAM_VAULT_KEY_EPHEMERAL, key, KEY_SIZE );
    if( OCKAM_ERR_NONE != status ) {
        log_error(status, "failed to generate static keypair in initiator_step_1");
        goto exit_block;
    }

    status = ockam_vault_key_get_pub( OCKAM_VAULT_KEY_EPHEMERAL, p_h->e, KEY_SIZE );
    if( OCKAM_ERR_NONE != status ) {
        log_error(status, "failed to generate get static public key in initiator_step_1");
        goto exit_block;
    }

    // Nonce to 0, k to empty
    p_h->nonce = 0;
    memset(p_h->k, 0, sizeof(p_h->k));

    // Initialize h
    memset( &p_h->h[0], 0, SHA256_SIZE );
    memcpy( &p_h->h[0], PROTOCOL_NAME, PROTOCOL_NAME_SIZE );

    // Initialize ck
    memset( &p_h->ck[0], 0, KEY_SIZE );
    memcpy( &p_h->ck[0], PROTOCOL_NAME, PROTOCOL_NAME_SIZE );

    // h = SHA256(h || prologue), prologue is empty
    mix_hash( p_h, NULL, 0 );

exit_block:
    return status;
}

/**
 ********************************************************************************************************
 *                                          test_responder_handshake()
 ********************************************************************************************************
 *
 * Summary: Test the handshake process by starting with predefined static and ephemeral keys
 *          (generated in the prologue) and verifying intermediate results against test data
 *          along the way
 *
 * @param connection [in] - initialized transport connection
 * @param p_h [in/out] - pointer to handshake structure
 * @return [out] - OCKAM_ERR_NONE on success
 ********************************************************************************************************
 */
OCKAM_ERR test_responder_handshake( OCKAM_TRANSPORT_CONNECTION connection, XX_HANDSHAKE* p_h )
{
    OCKAM_ERR                       status = OCKAM_ERR_NONE;
    uint8_t                         send_buffer[MAX_TRANSMIT_SIZE];
    uint8_t                         recv_buffer[MAX_TRANSMIT_SIZE];
    uint16_t                        transmit_size = 0;
    uint16_t                        bytes_received = 0;
    uint8_t                         compare[1024];
    uint32_t                        compare_bytes;

    /* Prologue initializes keys and handshake parameters */
    status = test_responder_prologue( p_h );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "test_handshake_prologue failed");
        goto exit_block;
    }
    /* Msg 1 receive */
    status = ockam_receive_blocking( connection, &recv_buffer[0], MAX_TRANSMIT_SIZE, &bytes_received );
    if(status != OCKAM_ERR_NONE) {
        log_error( status, "ockam_receive_blocking for msg 1 failed" );
        goto exit_block;
    }

    /* Msg 1 process */
    status = xx_responder_m1_process( p_h, recv_buffer, bytes_received );
    if(status != OCKAM_ERR_NONE) {
        log_error( status, "responder_m1_receive failed" );
        goto exit_block;
    }

    /* Msg 2 make */
    status = xx_responder_m2_make( p_h, send_buffer, sizeof(send_buffer), &transmit_size );
    if(status != OCKAM_ERR_NONE) {
        log_error( status, "responder_m2_send failed" );
        goto exit_block;
    }

    /* Msg 2 verify */
    string_to_hex( MSG_2_CIPHERTEXT, compare, &compare_bytes );
    if( 0 != memcmp( send_buffer, compare, compare_bytes )) {
        printf("Test failed on msg 2\n");
        goto exit_block;
    }

    /* Msg 2 send */
    status = ockam_send_blocking( connection, send_buffer, transmit_size );
    if(status != OCKAM_ERR_NONE) {
        log_error( status, "responder_m2_send failed" );
        goto exit_block;
    }

    /* Msg 3 receive */
    status = ockam_receive_blocking( connection, recv_buffer, MAX_TRANSMIT_SIZE, &bytes_received );
    if(status != OCKAM_ERR_NONE) {
        log_error( status, "ockam_receive_blocking failed for msg 3" );
        goto exit_block;
    }

    /* Msg 3 process */
    status = xx_responder_m3_process( p_h, recv_buffer,  bytes_received );
    if(status != OCKAM_ERR_NONE) {
        log_error( status, "responder_m3_process failed for msg 3" );
        goto exit_block;
    }

    /* Epilogue */
    status = xx_responder_epilogue( p_h );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "Failed responder_epilogue" );
        goto exit_block;
    }

exit_block:
    return status;
}
/**
 ********************************************************************************************************
 *                                          get_ip_info()
 ********************************************************************************************************
 *
 * Summary: Utility function to read IP address from text file located in config directory
 *
 * @param p_address [out] - if successful, holds OCKAM_INTERNET_ADDRESS for IP connection
 * @return [out] - OCKAM_ERR_NONE on success
 *                                          get_ip_info()
 ********************************************************************************************************
 */
 #define DEFAULT_IP_ADDRESS "127.0.0.1"
 #define DEFAULT_IP_PORT 8000

 OCKAM_ERR get_ip_info( int argc, char*argv[], OCKAM_INTERNET_ADDRESS* p_address )
 {
     OCKAM_ERR status = OCKAM_ERR_NONE;

     memset( p_address, 0, sizeof( *p_address));

     if( 3 != argc ) {
         strcpy( p_address->ip_address, DEFAULT_IP_ADDRESS );
         p_address->port = DEFAULT_IP_PORT;
     } else {
         strcpy( p_address->ip_address, argv[1] );
         p_address->port = strtoul( argv[2], NULL, 0 );
     }

 exit_block:
     return status;
 }

/**
 ********************************************************************************************************
 *                                   establish_responder_connection()
 ********************************************************************************************************
 *
 * Summary:
 *
 * @param p_listener
 * @param p_connection
 * @return
 */
OCKAM_ERR establish_responder_connection( int argc, char* argv[], OCKAM_TRANSPORT_CONNECTION* p_listener,
    OCKAM_TRANSPORT_CONNECTION* p_connection )
{
    OCKAM_ERR                       status = OCKAM_ERR_NONE;
    OCKAM_LISTEN_ADDRESS            listener_address;
    OCKAM_TRANSPORT_CONNECTION      connection = NULL;
    OCKAM_TRANSPORT_CONNECTION      listener = NULL;

    // Get the IP address to listen on
    status = get_ip_info( argc, argv, &listener_address.internet_address );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "failed to get address into");
        goto exit_block;
    }

    status = ockam_init_posix_tcp_connection( &listener );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "failed ockam_init_posix_tcp_connection");
        goto exit_block;
    }
    *p_listener = listener;

    // Wait for a connection
    status = ockam_listen_blocking( listener, &listener_address, &connection );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "listen failed" );
        goto exit_block;
    }

    *p_connection = connection;

exit_block:
    return status;
}

/**
 ********************************************************************************************************
 *                                   main()
 ********************************************************************************************************
 *
 * @return - 0 on success
 */
int main( int argc, char* argv[])
{
    OCKAM_ERR                       status = OCKAM_ERR_NONE;
    OCKAM_TRANSPORT_CONNECTION      connection = NULL;
    OCKAM_TRANSPORT_CONNECTION      listener = NULL;
    XX_HANDSHAKE                    handshake;
    uint8_t                         send_buffer[MAX_TRANSMIT_SIZE];
    uint8_t                         recv_buffer[MAX_TRANSMIT_SIZE];
    uint16_t                        transmit_size = 0;
    uint8_t                         test[16];
    uint32_t                        test_size;
    uint8_t                         test_initiator[TEST_MSG_BYTE_SIZE];
    uint8_t                         comp[2048];
    uint32_t                        comp_size;

    init_err_log(stdout);

    /*-------------------------------------------------------------------------
     * Establish transport connection with responder
     *-----------------------------------------------------------------------*/

     status = establish_responder_connection( argc, argv, &listener, &connection );
     if( OCKAM_ERR_NONE != status ) {
         log_error(status, "Failed to establish connection with responder");
         goto exit_block;
     }

     /*-------------------------------------------------------------------------
     * Initialize vault
     *-----------------------------------------------------------------------*/

     status = ockam_vault_init((void*) &vault_cfg);
     if(status != OCKAM_ERR_NONE) {
         log_error( status, "ockam_vault_init failed" );
         goto exit_block;
     }

     /*-------------------------------------------------------------------------
     * Perform the secret handshake
     * If successful, encrypt/decrypt keys will be established
     *-----------------------------------------------------------------------*/

     status = test_responder_handshake( connection, &handshake );
     if( OCKAM_ERR_NONE != status ) {
         log_error( status, "ockam_responder_handshake failed" );
         goto exit_block;
     }

     /*-------------------------------------------------------------------------
     * Verify secure channel by sending and receiving a known message
     *-----------------------------------------------------------------------*/

     /* Convert string to hex bytes and encrypt */
     string_to_hex(TEST_MSG_RESPONDER, test, &test_size );
     status = encrypt( &handshake, test, test_size,
         send_buffer, sizeof(send_buffer), &transmit_size );
    if(status != OCKAM_ERR_NONE) {
        log_error( status, "responder_epilogue_make failed" );
        goto exit_block;
    }
    /* Verify test message ciphertext */
    string_to_hex( MSG_4_CIPHERTEXT, comp, &comp_size );
    if( 0 != memcmp(comp, send_buffer, transmit_size)) {
        status = OCKAM_ERR_XX_HANDSHAKE_TEST_FAILED;
        log_error(status, "Msg 4 failed");
        goto exit_block;
    }

    /* Send test message */
    status = ockam_send_blocking( connection, send_buffer, transmit_size );
    if(status != OCKAM_ERR_NONE) {
        log_error( status, "ockam_send_blocking epilogue failed" );
        goto exit_block;
    }

    /* Receive test message  */
    status = ockam_receive_blocking( connection, recv_buffer, MAX_TRANSMIT_SIZE, &transmit_size );
    if(status != OCKAM_ERR_NONE) {
        log_error( status, "ockam_receive_blocking failed for msg 3" );
        goto exit_block;
    }

    /* Decrypt test message */

    status = decrypt( &handshake, test, TEST_MSG_BYTE_SIZE, recv_buffer, transmit_size, &test_size );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "ockam_receive_blocking failed on msg 2" );
        goto exit_block;
    }

    /* Verify test message */

    string_to_hex( TEST_MSG_INITIATOR, test_initiator, NULL );
    if( 0 != memcmp( (void*)test, test_initiator, TEST_MSG_BYTE_SIZE) ){
        status = OCKAM_ERR_XX_HANDSHAKE_FAILED;
        log_error( status, "Received bad test message" );
        goto exit_block;
    }

exit_block:
    if( NULL != connection ) ockam_uninit_connection( connection );
    if( NULL != listener ) ockam_uninit_connection( listener );
    printf( "Test ended with status %0.4x\n", status );
    return status;
}
