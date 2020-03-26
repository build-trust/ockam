/**
 ********************************************************************************************************
 * @file        posixSocket.h
 * @brief       Defines common code between TCP/UDP sockets
 ********************************************************************************************************
 */
#ifndef PosixSocket_H
#define PosixSocket_H 1

#include <arpa/inet.h>
#include <errno.h>
#include <netdb.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <time.h>

#include "ockam/transport.h"

/*
 ********************************************************************************************************
 *                                                DEFINES *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                        PUBLIC DATA TYPES *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                        PRIVATE DATA TYPES *
 ********************************************************************************************************
 */

/*
 ********************************************************************************************************
 *                                          FUNCTION PROTOTYPES *
 ********************************************************************************************************
 */

/**
 * MakeSocketAddress - construct network-friendly address from user-friendly
 * address
 * @param p_ip_address - (in) IP address in "nnn.nnn.nnn.nnn" format
 * @param port - port number, must be non-zero
 * @param p_socketAddress - (out) address converted
 * @return - OCKAM_NO_ERR on success
 */
TransportError MakeSocketAddress(char *p_ip_address, in_port_t port, struct sockaddr_in *p_socketAddress);

#endif
