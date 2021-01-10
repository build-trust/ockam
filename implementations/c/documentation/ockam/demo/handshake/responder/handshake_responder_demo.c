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
int main( int argc, char* argv[] ) {
    OCKAM_ERR                       status = OCKAM_ERR_NONE;
    OCKAM_TRANSPORT_CONNECTION      connection = NULL;
    OCKAM_TRANSPORT_CONNECTION      listener = NULL;
    XX_HANDSHAKE                    handshake;
    uint8_t                         send_buffer[MAX_TRANSMIT_SIZE];
    uint8_t                         recv_buffer[MAX_TRANSMIT_SIZE];
    uint16_t                        transmit_size = 0;
    uint8_t                         msg[80];
    char*                           p_msg = (char*)msg;
    size_t                          msg_size = 80;

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

    status = ockam_xx_responder_handshake( connection, &handshake );
    if( OCKAM_ERR_NONE != status ) {
    log_error( status, "ockam_responder_handshake failed" );
    goto exit_block;
    }

    /*-------------------------------------------------------------------------
     * Demo loop - get input, encrypt, send, receive, decrypt
     *-----------------------------------------------------------------------*/

    do {
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
    status = ockam_receive_blocking( connection, recv_buffer, MAX_TRANSMIT_SIZE, &transmit_size );
    if(status != OCKAM_ERR_NONE) {
    log_error( status, "ockam_receive_blocking failed for msg 3" );
    goto exit_block;
    }
    print_uint8_str( recv_buffer, transmit_size, "\nReceived ciphertext: ");
    status = decrypt( &handshake, msg, 80, recv_buffer, transmit_size, &msg_size );
    if( OCKAM_ERR_NONE != status ) {
    log_error( status, "ockam_receive_blocking failed on msg 2" );
    goto exit_block;
    }
    printf("\nDecrypted: %s\n", msg);

    } while( msg[0] != 'q' );


exit_block:
    if( NULL != connection ) ockam_uninit_connection( connection );
    if( NULL != listener ) ockam_uninit_connection( listener );
    printf( "Test ended with status %0.4x\n", status );
    return status;
}
