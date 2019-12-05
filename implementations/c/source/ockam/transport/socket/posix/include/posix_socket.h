#ifndef POSIX_SOCKET_H
#define POSIX_SOCKET_H 1

#include "transport.h"

/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES                                         *
 ********************************************************************************************************
 */

/**
 * make_socket_address - construct network-friendly address from user-friendly address
 * @param p_ip_address - (in) IP address in "nnn.nnn.nnn.nnn" format
 * @param port - port number, must be non-zero
 * @param p_socket_address - (out) address converted
 * @return
 */
OCKAM_ERR make_socket_address( char* p_ip_address, in_port_t port, struct sockaddr_in* p_socket_address );


#endif