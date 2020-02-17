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

int main(int argc, char* argv[]) {
    OCKAM_ERR                       status = OCKAM_ERR_NONE;
    OCKAM_TRANSPORT_CONNECTION      connection;
    XX_HANDSHAKE                    handshake;
    uint8_t                         send_buffer[MAX_TRANSMIT_SIZE];
    uint8_t                         recv_buffer[MAX_TRANSMIT_SIZE];
    uint16_t                        transmit_size = 0;
    uint32_t                        test_size;
    uint8_t                         msg[80];
    char*                           p_msg = (char*)msg;
    size_t                          msg_size;

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
    status = ockam_xx_initiator_handshake( connection, &handshake );
    if( OCKAM_ERR_NONE != status ) {
    log_error( status, "ockam_initiator_handshake" );
    goto exit_block;
    }

    /*-------------------------------------------------------------------------
     * Demo loop - get input, encrypt, send, receive, decrypt
     *-----------------------------------------------------------------------*/

    do {
    status = ockam_receive_blocking( connection, recv_buffer, MAX_TRANSMIT_SIZE, &transmit_size );
    if(status != OCKAM_ERR_NONE) {
    log_error( status, "ockam_receive_blocking failed for msg 3" );
    goto exit_block;
    }
    print_uint8_str( recv_buffer, transmit_size, "\nReceived ciphertext: ");
    status = decrypt( &handshake, msg, 80, recv_buffer, transmit_size, &test_size );
    if( OCKAM_ERR_NONE != status ) {
    log_error( status, "ockam_receive_blocking failed on msg 2" );
    goto exit_block;
    }
    printf("\nDecrypted: %s\n", msg);
    msg_size = 80;
    printf( "Type a message: \n" );
    getline( &p_msg, &msg_size, stdin );
    status = encrypt( &handshake, msg, msg_size, send_buffer, sizeof( send_buffer ), &transmit_size);
    if(status != OCKAM_ERR_NONE) {
    log_error( status, "responder_epilogue_make failed" );
    goto exit_block;
    }
    print_uint8_str( send_buffer, transmit_size, "\nCiphertext:\n");
    status = ockam_send_blocking( connection, send_buffer, transmit_size );
    if(status != OCKAM_ERR_NONE) {
    log_error( status, "ockam_send_blocking epilogue failed" );
    goto exit_block;
    }

    } while( msg[0] != 'q' );

exit_block:
    if( NULL != connection ) ockam_uninit_connection( connection );
    printf( "Test ended with status %0.4x\n", status );
    return status;
}
