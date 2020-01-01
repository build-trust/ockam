//
// Created by Robin Budd on 191227.
//
#include <stdio.h>
#include "transport.h"
#include "connection.h"
#include "error.h"
#include "syslog.h"

OCKAM_ERR ockam_listen_blocking( OCKAM_TRANSPORT_CONNECTION listener,
                                 OCKAM_LISTEN_ADDRESS* address, OCKAM_TRANSPORT_CONNECTION* connection )
{
	OCKAM_ERR           status = OCKAM_ERR_NONE;
	CONNECTION*         p_listener_connection = (CONNECTION*)listener;

	if( NULL == p_listener_connection ) {
		status = OCKAM_ERR_INVALID_PARAM;
		log_error( status, "listener connection must be initialized inockam_listen_blocking ");
		goto exit_block;
	}

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
