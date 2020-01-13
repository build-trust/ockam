/**
 ********************************************************************************************************
 * @file        transport.c
 * @brief       Transport.c implements the outward-facing API calls for the transport layer. Functions
 *              in this file are mostly pass-through. They do minimal parameter checking,
 *              then dispatch to the appropriate function for the specific connection type,
 *              which is obtained from the function dispatch table (AKA the interface).
 *              See "connection.h" for details.
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES                                            *
 ********************************************************************************************************
 */

#include <stdio.h>
#include "ockam/transport.h"
#include "./include/connection.h"
#include "ockam/error.h"
#include "ockam/syslog.h"

OCKAM_ERR ockam_listen_blocking( OCKAM_TRANSPORT_CONNECTION listener,
                                 OCKAM_LISTEN_ADDRESS* address, OCKAM_TRANSPORT_CONNECTION* connection )
{
	OCKAM_ERR           status = OCKAM_ERR_NONE;
	CONNECTION*         p_listener_connection = (CONNECTION*)listener;

	// Basic parameter evaluation
	if( NULL == p_listener_connection ) {
		status = OCKAM_ERR_INVALID_PARAM;
		log_error( status, "listener connection must be initialized in ockam_listen_blocking ");
		goto exit_block;
	}

	// Dispatch
	status = p_listener_connection->p_interface->listen_blocking( listener, address, connection );

exit_block:
	return status;
}

OCKAM_ERR ockam_uninit_connection( OCKAM_TRANSPORT_CONNECTION connection )
{
	OCKAM_ERR           status = OCKAM_ERR_NONE;
	CONNECTION*         p_connection = (CONNECTION*)connection;

	if( NULL == p_connection ) {
		status = OCKAM_ERR_INVALID_PARAM;
		log_error( status, "listener connection must be initialized inockam_listen_blocking ");
		goto exit_block;
	}

	status = p_connection->p_interface->uninitialize( connection );

exit_block:
	return status;

}

OCKAM_ERR ockam_connect_blocking( void* address, OCKAM_TRANSPORT_CONNECTION* connection )
{
	OCKAM_ERR           status = OCKAM_ERR_NONE;
	CONNECTION*         p_connection = (CONNECTION*)connection;

	if( NULL == p_connection ) {
		status = OCKAM_ERR_INVALID_PARAM;
		log_error( status, "connection must be initialized ockam_connect_blocking ");
		goto exit_block;
	}

	if( NULL == address ) {
		status = OCKAM_ERR_INVALID_PARAM;
		log_error( status, "host address required for connection in ockam_connect_blocking");
		goto exit_block;
	}

	status = p_connection->p_interface->connect_blocking( address, connection );

exit_block:
	return status;
}

OCKAM_ERR ockam_send_blocking( OCKAM_TRANSPORT_CONNECTION connection,
                               void* p_buffer, uint16_t size )
{
	OCKAM_ERR           status = OCKAM_ERR_NONE;
	CONNECTION*         p_connection = (CONNECTION*)connection;

	if( NULL == p_connection ) {
		status = OCKAM_ERR_INVALID_PARAM;
		log_error( status, "connection must be initialized ockam_connect_blocking ");
		goto exit_block;
	}

	status = p_connection->p_interface->send_blocking( connection, p_buffer, size );

exit_block:
	return status;
}

OCKAM_ERR ockam_receive_blocking( OCKAM_TRANSPORT_CONNECTION connection,
                                  void* p_buffer, uint16_t size, uint16_t* p_bytes_received )
{
	OCKAM_ERR           status = OCKAM_ERR_NONE;
	CONNECTION*         p_connection = (CONNECTION*)connection;

	if( NULL == p_connection ) {
		status = OCKAM_ERR_INVALID_PARAM;
		log_error( status, "connection must be initialized ockam_connect_blocking ");
		goto exit_block;
	}

	status = p_connection->p_interface->receive_blocking( connection, p_buffer, size, p_bytes_received );

exit_block:
	return status;
}
