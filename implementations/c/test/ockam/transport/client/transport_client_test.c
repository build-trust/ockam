#include <stdio.h>
#include <string.h>
#include "ockam/transport.h"
#include "ockam/syslog.h"

char* p_file_to_send =  "../test_data.txt";
char* p_file_to_receive = "./test_data_received.txt";
char* p_file_to_compare = "../test_data.txt";

OCKAM_ERR file_compare( char* p_f1, char* p_f2 )
{
    OCKAM_ERR    status = 0;

    uint16_t    more = 1;

    FILE*       fp1 = NULL;
    FILE*       fp2 = NULL;

    char        buffer1[256];
    char        buffer2[256];

    size_t      r1;
    size_t      r2;

    fp1 = fopen( p_f1, "r" );
    fp2 = fopen( p_f2, "r" );

    if( (NULL == fp1) || (NULL == fp2)) {
        status = OCKAM_ERR_TRANSPORT_TEST;
        goto exit_block;
    }

    while( more ) {
        r1 = fread( buffer1, 1, sizeof( buffer1 ), fp1 );
        r2 = fread( buffer2, 1, sizeof( buffer2 ), fp2 );
        if( r1 != r2 ) {
            status = OCKAM_ERR_TRANSPORT_TEST;
            goto exit_block;
        }
        if( 0 != memcmp( buffer1, buffer2, r1 )) {
            status = OCKAM_ERR_TRANSPORT_TEST;
            goto exit_block;
        }
        if( feof(fp1) ) {
            if(!feof(fp2)) {
                status = OCKAM_ERR_TRANSPORT_TEST;
                goto exit_block;
            }
            more = 0;
        }
    }

exit_block:
    if( NULL != fp1 ) fclose( fp1 );
    if( NULL != fp2 ) fclose( fp2 );
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

int main( int argc, char* argv[]) {

    OCKAM_ERR                       status              = OCKAM_ERR_NONE;
    OCKAM_TRANSPORT_CONNECTION      connection          = NULL;
    OCKAM_LISTEN_ADDRESS            host_address;
    char                            send_buffer[64];
    uint16_t                        send_length;
    char                            receive_buffer[64];
    uint16_t                        bytes_received      = 0;
    FILE*                           file_send           = NULL;
    FILE*                           file_receive        = NULL;
    size_t                          bytes_written;
    uint16_t                        send_not_done       = 1;
    uint16_t                        receive_not_done    = 1;

    init_err_log( stdout );

    status = get_ip_info( argc, argv, &host_address.internet_address );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "failed to get address into");
        goto exit_block;
    }

    // Open the test data file for sending
    file_send = fopen( p_file_to_send, "r" );
    if( NULL == file_send ) {
        status = OCKAM_ERR_TRANSPORT_TEST;
        log_error( status, "failed to open test file test_data_client.txt");
        goto exit_block;
    }

    // Create file for test data received
    file_receive = fopen( p_file_to_receive, "w" );
    if( NULL == file_receive ) {
        status = OCKAM_ERR_TRANSPORT_TEST;
        log_error( status, "failed to open test file test_data_client.txt");
        goto exit_block;
    }

    // Initialize TCP connection
    status = ockam_init_posix_tcp_connection( &connection );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "failed ockam_init_posix_tcp_connection");
        goto exit_block;
    }

    // Try to connect
    status = ockam_connect_blocking( &host_address, connection );
    if( OCKAM_ERR_NONE != status ) {
        log_error( status, "connect failed" );
        goto exit_block;
    }

    // Send the test data file
    while( send_not_done ) {
        send_length = fread( &send_buffer[0], 1, sizeof( send_buffer ), file_send );
        if ( feof( file_send )) send_not_done = 0;
        status = ockam_send_blocking( connection, &send_buffer[0], send_length );
        if ( OCKAM_ERR_NONE != status ) {
            log_error( status, "Send failed" );
            goto exit_block;
        }
    }
    // Send special "the end" buffer

    status = ockam_send_blocking( connection, "that's all", strlen("that's all")+1 );
    if ( OCKAM_ERR_NONE != status ) {
        log_error( status, "Send failed" );
        goto exit_block;
    }

    // Receive the test data file
    while( receive_not_done ) {
        status = ockam_receive_blocking( connection, &receive_buffer[0], sizeof( receive_buffer ), &bytes_received );
        if ( OCKAM_ERR_NONE != status ) {
            log_error( status, "Receive failed" );
            goto exit_block;
        }
        // Look for special "the end" buffer
        if ( 0 == strncmp( "that's all", &receive_buffer[0], strlen( "that's all" ))) {
            receive_not_done = 0;
        } else {
            bytes_written = fwrite( &receive_buffer[0], 1, bytes_received, file_receive );
            if ( bytes_written != bytes_received ) {
                log_error( OCKAM_ERR_TRANSPORT_TEST, "failed write to output file" );
                goto exit_block;
            }
        }
    }

    fclose( file_send );
    fclose( file_receive );

    // Now compare the received file and the reference file
    if( 0 != file_compare( p_file_to_receive, p_file_to_compare )) {
        status = OCKAM_ERR_TRANSPORT_TEST;
        log_error( status, "file compare failed" );
        goto exit_block;
    }


exit_block:
    if( NULL != connection ) ockam_uninit_connection( connection );
    return status;
}
