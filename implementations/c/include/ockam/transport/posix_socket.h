/**
 ********************************************************************************************************
 * @file        posix_socket.h
 * @brief       Defines common code between TCP/UDP sockets
 ********************************************************************************************************
 */
#ifndef POSIX_SOCKET_H
#define POSIX_SOCKET_H 1

#include <errno.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/tcp.h>
#include <arpa/inet.h>
#include <netinet/in.h>
#include <netdb.h>
#include <errno.h>
#include <time.h>
#include "ockam/transport.h"

/*
 ********************************************************************************************************
 *                                                DEFINES                                               *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                        PUBLIC DATA TYPES                                             *
 ********************************************************************************************************
 */


/*
 ********************************************************************************************************
 *                                        PRIVATE DATA TYPES                                            *
 ********************************************************************************************************
 */

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
 * @return - OCKAM_NO_ERR on success
 */
OCKAM_ERR make_socket_address( char* p_ip_address, in_port_t port,
    struct sockaddr_in* p_socket_address );


#endif
