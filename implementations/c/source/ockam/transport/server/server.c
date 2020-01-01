#include "server.h"

/*
 * 1. Initialization. Allocate the structs. X
 * 2. Spin up event processing thread. X
 * 3. ---event processing loop---
 */
//
//OCKAM_ERR ockam_init_transport_server( uint16_t max_connections, OCKAM_TRANSPORT_SERVER* server )
//{
//	TRANSPORT_SERVER*       p_server = NULL;
//	int                     status = 0;
//
//	if(( 0 == max_connections ) || ( NULL == server )) {
//		status = OCKAM_ERR_INVALID_PARAM;
//		log_error( status, "invalid parameter in ockam_init_transport_server");
//		goto exit_block;
//	}
//
//	p_server = malloc( sizeof(TRANSPORT_SERVER) + ((max_connections-1)*sizeof( OCKAM_TRANSPORT_CONNECTION ));
//	if( NULL == p_server ) {
//		status = OCKAM_ERR_MEM_INSUFFICIENT;
//		log_error( status, "no memory in ockam_init_transport_server" );
//		goto exit_block;
//	}
//	memset( p_server, 0, sizeof((TRANSPORT_SERVER) + ((max_connections-1)*sizeof( OCKAM_TRANSPORT_CONNECTION ))));
//
//	p_server->max_connections = max_connections;
//	*server = p_server;
//
//exit_block:
//	if( OCKAM_ERR_NONE != status ) {
//		if( NULL != p_server ) {
//			free( p_server );
//		}
//	}
//	return status;
//}
//
//OCKAM_ERR ockam_uninit_transport_server( TRANSPORT_SERVER* p_server )
//{
//
//	free (p_server);
//	return 0;
//}

/*
 * 2. Listen.
 *  a. Spin up a thread.
 *  b. For each accept up to n
 *      Callback with connection & context
 *  c. Once we have n connections
 *      Wait for a disconnect
 */

/*
 * 3. Read.
 *  a. Queue read event.
 *  b. If blocking, wait until read completes.
 */

/*
 * 4. Write.
 *  a. Queue write event.
 *  b. If blocking, wait until write completes.
 */
