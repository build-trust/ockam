#include <stdlib.h>
#include <stdio.h>
#include "transport.h"
#include "error.h"
#include "syslog.h"

/**
 * ockam_xp_send - Sends a buffer to the host server (blocking)
 * @param handle - (in) Handle to initilized client connection
 * @param buffer - (in) Pointer to buffer to be sent
 * @param length - (in) Number of bytes to send
 * @param p_bytes_sent - (out) Number of bytes successfully sent
 * @return - OCKAM_SUCCESS or an error code
 */
//OCKAM_ERR ockam_send(OCKAM_TRANSPORT handle,
//                     void* p_buffer, unsigned int length, unsigned int* p_bytes_sent) {
//
//	SOCKET_TYPE*        p_type          = (SOCKET_TYPE*)handle;
//	OCKAM_ERR           status          = OCKAM_ERR_NONE;
//
//	switch( *p_type ) {
//		case POSIX_TCP_CLIENT: {
//			status = posix_socket_tcp_send( handle, p_buffer, length, p_bytes_sent );
//			break;
//		}
//		case POSIX_UDP_CLIENT: {
//			status = posix_socket_udp_send( handle, p_buffer, length, p_bytes_sent );
//			break;
//		}
//		case POSIX_UDP_SERVER:
//		case POSIX_TCP_SERVER: {
//
//		}
//		default: {
//			log_error("not yet implemented in ockam_send");
//		}
//	}
//	return status;
//}
//
///**
// * ockam_receive - Receive a buffer over an initialized transport instance
// * @param handle - (in) Handle to initilized transport instance
// * @param buffer - (in) Pointer to receive buffer
// * @param length - (in) Size of receive buffer
// * @param p_bytes_received  - (out) Number of bytes received
// * @return - OCKAM_SUCCESS or an error code
// */
//OCKAM_ERR ockam_receive( OCKAM_TRANSPORT handle,
//		void* p_buffer, unsigned int length, unsigned int* p_bytes_received)
//{
//	SOCKET_TYPE*        p_type          = (SOCKET_TYPE*)handle;
//	OCKAM_ERR           status          = OCKAM_ERR_NONE;
//
//	switch( *p_type ) {
//		case POSIX_TCP_CLIENT: {
//			status = posix_socket_tcp_receive( handle, p_buffer, length, p_bytes_received );
//			break;
//		}
//		case POSIX_UDP_SERVER: {
//			status = posix_socket_udp_receive( handle, p_buffer, length, p_bytes_received );
//			break;
//		}
//		case POSIX_TCP_SERVER: {
//			status = posix_socket_tcp_receive( handle, p_buffer, length, p_bytes_received );
//			break;
//		}
//		default: {
//			log_error("not yet implemented in ockam_receive");
//		}
//	}
//
//	return status;
//}
//
///**
// * ockam_uninit_transport -  Closes connection and frees resources
// * @param handle - Handle of initialized transport instance
// * @return  - OCKAM_SUCCESS or error
// */
//OCKAM_ERR ockam_uninit_transport( OCKAM_TRANSPORT handle )
//{
//	SOCKET_TYPE*        p_type          = (SOCKET_TYPE*)handle;
//	OCKAM_ERR           status          = OCKAM_ERR_NONE;
//
//	switch( *p_type ) {
//		case POSIX_TCP_CLIENT: {
//			status = uninit_posix_socket_tcp_client( handle );
//			break;
//		}
//		case POSIX_TCP_SERVER: {
//			status = uninit_posix_socket_tcp_server( handle );
//			break;
//		}
//		case POSIX_UDP_SERVER:
//		case POSIX_UDP_CLIENT: {
//			status = uninit_posix_socket_udp( handle );
//			break;
//		}
//		default: {
//			log_error("not yet implemented in ockam_uninit_connection");
//		}
//	}
//
//	return status;
//}

/**
 * make_socket_address - constructs network-ready socket address from user-friendly format
 * @param p_ip_address - (in) pointer to IP address string in nnn.nnn.nnn format
 * @param port  - (in) port number in local machine byte order
 * @param p_socket_address - (out) network-ready sockaddr_in structure
 * @return - OCKAM_ERR_NONE if successful
 */
OCKAM_ERR make_socket_address( char* p_ip_address, in_port_t port, struct sockaddr_in* p_socket_address )
{
	OCKAM_ERR       status      = OCKAM_ERR_NONE;
	int             in_status    = 0;

	// Get the host IP address and port
	p_socket_address->sin_family = AF_INET;
	p_socket_address->sin_port = htons(port);
	if( NULL != p_ip_address ) {
		in_status = inet_pton( AF_INET,
		                       p_ip_address,
		                       &p_socket_address->sin_addr.s_addr );
		if(1 != in_status){
			log_error( status, "inet_pton failed in make_socket_address" );
			status = OCKAM_ERR_TRANSPORT_ADDRESS;
			goto exit_block;
		}
	} else {
		p_socket_address->sin_addr.s_addr = htonl (INADDR_ANY);
	}

exit_block:
	return status;
}

