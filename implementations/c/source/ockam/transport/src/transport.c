#include <stdlib.h>
#include <stdio.h>
#include <fcntl.h>
#include <unistd.h>
#include "ockam_transport.h"
#include "transport.h"
#include "errlog.h"
#include <string.h>

//OCKAM_ERROR init_tcp_socket( TCP_CONNECTION* p_tcp,
//		OCKAM_DEVICE_RECORD* p_device ) {
//	OCKAM_ERROR				status			= OCKAM_SUCCESS;
//
//	// Get local and host addresses from device record and
//	// convert to socket-ready format. This assumes that there are
//	// well-formed IP addresses in the device record. #revisit
//	if( 1 != inet_aton( &p_device->local_address.ip_address[0],
//		&p_tcp->socket_address_local.sin_addr )) {
//			log_error("inet_aton failed in ockam_xp_init_client");
//			status = OCKAM_ERR_INVALID_LOCAL_ADDRESS;
//			goto exit_block;
//	}
//    p_tcp->socket_address_local.sin_family = AF_INET;
//    //!!p_tcp->socket_address_local.sin_len = sizeof(p_tcp->socket_address_host);
//
//    // #revisit is there a better way?
//	if( strlen(&p_device->host_address.ip_address[0]) > 0 ) {
//        if (1 != inet_aton(&p_device->host_address.ip_address[0],
//                           &p_tcp->socket_address_host.sin_addr)) {
//            log_error("inet_aton failed in ockam_xp_init_client");
//            status = OCKAM_ERR_INVALID_REMOTE_ADDRESS;
//            goto exit_block;
//        }
//    }
//
//	// Create the socket
//	p_tcp->socket = socket( AF_INET, SOCK_STREAM, 0 );
//	if( -1 == p_tcp->socket ) {
//		log_error("failed to create socket");
//		status = OCKAM_ERR_INIT_TRANSPORT;
//		goto exit_block;
//	}
//
//exit_block:
//	return status;
//}
//
//OCKAM_ERROR read_from_client(TCP_RECEIVE* p_receive,
//	void* p_buffer,
//	unsigned int buffer_size,
//	unsigned int* p_bytes_read
//) {
//	OCKAM_ERROR		status = OCKAM_SUCCESS;
//	int				bytes_read = 0;
//	int				total_bytes = 0;
//	socklen_t		client_addr_size = 0;
//
//	// Wait for incoming connect request
//	if( 0 != listen( p_receive->listen_socket, 1 ) ) {
//		log_error("listen failed in wait_for_client");
//		status = OCKAM_ERR_RECEIVER;
//		goto exit_block;
//	}
//
//	// Accept the connection
//	client_addr_size = sizeof(p_receive->client_addr);
//	p_receive->receive_socket = accept( p_receive->listen_socket,
//		&p_receive->client_addr,
//		&client_addr_size);
//	if(-1 == p_receive->receive_socket){
//		log_error("accept failed in wait_for_client");
//		status = OCKAM_ERR_RECEIVER;
//		goto exit_block;
//	}
//
//	// Receive the data
//	do {
//		bytes_read = read(p_receive->receive_socket, p_buffer, buffer_size);
//		if( -1 == bytes_read ) {
//			log_error("read error in read_from_client");
//			status = OCKAM_ERR_RECEIVER;
//			goto exit_block;
//		}
//		total_bytes += bytes_read;
//		buffer_size -= bytes_read;
//		p_buffer += bytes_read;
//	} while( (bytes_read > 0) && (buffer_size > 0) );
//
//	// #revisit what if there's more data than buffer can accommodate
//
//	// Shut it down...
//	close(p_receive->receive_socket);
//
//exit_block:
//	*p_bytes_read = total_bytes;
//	return status;
//}
