
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

OCKAM_VAULT_CFG_s vault_cfg =
{
    .p_tpm                       = 0,
    .p_host                      = 0,
    OCKAM_VAULT_EC_CURVE25519
};

/**
 ********************************************************************************************************
 *                                          test_initiator_prologue()
 ********************************************************************************************************
 *
 * Summary: This differs from the production handshake_prologue in that it initiates the handshake
 *          with a known set of keys so that cipher results can be verified along the way.
 *
 * @param p_h [in/out] - pointer to handshake struct
 * @return [out] - OCKAM_ERR_NONE on success
 */

OCKAM_ERR test_initiator_prologue( XX_HANDSHAKE* p_h )
{
    OCKAM_ERR       status = OCKAM_ERR_NONE;
    uint8_t         key[KEY_SIZE];
    uint32_t        key_bytes;

    // 1. Pick a static 25519 keypair for this handshake and set it to s
    string_to_hex( INITIATOR_STATIC, key, &key_bytes );
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
    string_to_hex( INITIATOR_EPH, key, &key_bytes );
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

    // Initialize h to "Noise_XX_25519_AESGCM_SHA256" and set prologue to empty
    memset( &p_h->h[0], 0, SHA256_SIZE );
    memcpy( &p_h->h[0], PROTOCOL_NAME, PROTOCOL_NAME_SIZE );

    // Initialize ck
    memset( &p_h->ck[0], 0, SHA256_SIZE );
    memcpy( &p_h->ck[0], PROTOCOL_NAME, PROTOCOL_NAME_SIZE );

    // h = SHA256(h || prologue), prologue is empty
    mix_hash( p_h, NULL, 0 );

exit_block:
    return status;
}

/**
 ********************************************************************************************************
 *                                          test_initiator_handshake()
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
 OCKAM_ERR test_initiator_handshake( OCKAM_TRANSPORT_CONNECTION connection, XX_HANDSHAKE* p_h )
{
    OCKAM_ERR                       status = OCKAM_ERR_NONE;
    uint8_t                         send_buffer[MAX_TRANSMIT_SIZE];
    uint8_t                         recv_buffer[MAX_TRANSMIT_SIZE];
    uint16_t                        bytes_received = 0;
    uint16_t                        transmit_size = 0;
    uint8_t                         compare[1024];
    uint32_t                        compare_bytes;

    /* Prologue initializes keys and handshake parameters */
    status = test_initiator_prologue( p_h );
    if(status != OCKAM_ERR_NONE) {
        log_error( status, "test_initiator_prologue" );
        goto exit_block;
    }

    // Step 1 generate message
    status = xx_initiator_m1_make( p_h, send_buffer, MAX_TRANSMIT_SIZE, &transmit_size );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "initiator_step_1 failed" );
        goto exit_block;
    }

    // Verify
    string_to_hex( MSG_1_CIPHERTEXT, compare, &compare_bytes );
    if( 0 != memcmp(send_buffer, compare, compare_bytes)) {
        status = OCKAM_ERR_XX_HANDSHAKE_TEST_FAILED;
        log_error( status, "Test failed on msg 0\n");
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
        log_error( status, "ockam_receive_blocking failed on msg 2" );
        goto exit_block;
    }

    // Msg 3 make
    status = xx_initiator_m3_make( p_h, send_buffer, &transmit_size );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "initiator_m3_make failed" );
        goto exit_block;
    }

    /* Msg 3 verify */
    string_to_hex( MSG_3_CIPHERTEXT, compare, &compare_bytes);
    if( 0 != memcmp( compare, send_buffer, transmit_size)) {
        status = OCKAM_ERR_XX_HANDSHAKE_TEST_FAILED;
        log_error(status, "-------Msg 3 verify failed");
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


OCKAM_ERR establish_connection( int argc, char* argv[], OCKAM_TRANSPORT_CONNECTION* p_connection )
{
    OCKAM_ERR                       status = OCKAM_ERR_NONE;
    OCKAM_INTERNET_ADDRESS          responder_address;
    OCKAM_TRANSPORT_CONNECTION      connection = NULL;

    // Get the IP address of the responder
    status = get_ip_info( argc, argv, &responder_address );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "failed to get address into");
        goto exit_block;
    }

    status = ockam_init_posix_tcp_connection( &connection );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "failed ockam_init_posix_tcp_connection");
        goto exit_block;
    }
    // Try to connect
    status = ockam_connect_blocking( &responder_address, connection );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "connect failed" );
        goto exit_block;
    }

    *p_connection = connection;

exit_block:
    return status;
}

int main( int argc, char* argv[] ) {
    OCKAM_ERR                       status = OCKAM_ERR_NONE;
    OCKAM_TRANSPORT_CONNECTION      connection;
    XX_HANDSHAKE                       handshake;
    uint8_t                         send_buffer[MAX_TRANSMIT_SIZE];
    uint8_t                         recv_buffer[MAX_TRANSMIT_SIZE];
    uint16_t                        bytes_received = 0;
    uint16_t                        transmit_size = 0;
    uint8_t                         test[TEST_MSG_BYTE_SIZE];
    uint32_t                        test_size;
    uint32_t                        test_bytes;
    uint8_t                         test_responder[TEST_MSG_BYTE_SIZE];
    char                            user_msg[80];
    uint8_t*                        p_user_msg = (uint8_t*)user_msg;
    uint32_t                        user_bytes;

    init_err_log(stdout);

    /*-------------------------------------------------------------------------
    * Establish transport connection with responder
    *-----------------------------------------------------------------------*/
    status = establish_connection( argc, argv, &connection );
    if( OCKAM_ERR_NONE != status ) {
        log_error(status, "Failed to establish connection with responder");
        goto exit_block;
    }
    status = ockam_vault_init((void*) &vault_cfg);                 /* Initialize vault                                   */
    if(status != OCKAM_ERR_NONE) {
        log_error( status, "ockam_vault_init failed" );
        goto exit_block;
    }

    /*-------------------------------------------------------------------------
     * Secure the connection
     *-----------------------------------------------------------------------*/
     status = test_initiator_handshake( connection, &handshake );
     if( OCKAM_ERR_NONE != status ) {
         log_error( status, "ockam_initiator_handshake" );
         goto exit_block;
     }

     /*-------------------------------------------------------------------------
     * Receive the test message
     *-----------------------------------------------------------------------*/
     status = ockam_receive_blocking( connection, recv_buffer, sizeof(recv_buffer), &bytes_received );
     if( OCKAM_ERR_NONE != status ) {
         log_error( status, "ockam_receive_blocking failed on msg 2" );
         goto exit_block;
     }

     /*-------------------------------------------------------------------------
     * Confirm the test message
     *-----------------------------------------------------------------------*/
     status = decrypt( &handshake, test, TEST_MSG_BYTE_SIZE, recv_buffer, bytes_received,  &test_bytes );
     if( OCKAM_ERR_NONE != status ) {
         log_error( status, "ockam_receive_blocking failed on msg 2" );
         goto exit_block;
     }
     string_to_hex( TEST_MSG_RESPONDER, test_responder, NULL );
     if( 0 != memcmp( (void*)test, test_responder, TEST_MSG_BYTE_SIZE) ){
         status = OCKAM_ERR_XX_HANDSHAKE_FAILED;
         log_error( status, "Received bad epilogue message" );
         goto exit_block;
     }

     /*-------------------------------------------------------------------------
     * Make the test message
     *-----------------------------------------------------------------------*/
     string_to_hex(TEST_MSG_INITIATOR, test, &test_bytes );
     status = encrypt( &handshake, test, test_bytes, send_buffer, sizeof(send_buffer), &transmit_size) ;
     if( OCKAM_ERR_NONE != status ) {
         log_error( status, "initiator_encrypt failed on test message" );
         goto exit_block;
     }

     /*-------------------------------------------------------------------------
     * Confirm the test message
     *-----------------------------------------------------------------------*/
     string_to_hex( MSG_5_CIPHERTEXT, test, &test_bytes);
     if( 0 != memcmp(test, send_buffer, transmit_size)) {
         status = OCKAM_ERR_XX_HANDSHAKE_TEST_FAILED;
         log_error(status, "Msg 5 failed");
         goto exit_block;
     }

     /*-------------------------------------------------------------------------
     * Send the test message
     *-----------------------------------------------------------------------*/
     status = ockam_send_blocking( connection, send_buffer, transmit_size );
     if( OCKAM_ERR_NONE != status ) {
         log_error( status, "ockam_send_blocking failed on test message" );
         goto exit_block;
     }

exit_block:
    if( NULL != connection ) ockam_uninit_connection( connection );
    printf( "Test ended with status %0.4x\n", status );
    return status;
}
