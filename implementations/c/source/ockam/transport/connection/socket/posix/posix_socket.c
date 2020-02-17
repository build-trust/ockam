#include <stdlib.h>
#include <stdio.h>
#include "ockam/transport.h"
#include "posix_socket.h"
#include "ockam/error.h"
#include "ockam/syslog.h"



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
