/**
 ********************************************************************************************************
 * @file        server.h
 * @brief       Defines server-specific data and functions
 ********************************************************************************************************
 */

#ifndef SERVER_TRANSPORT_SERVER_H
#define SERVER_TRANSPORT_SERVER_H

#include <ntsid.h>
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/tcp.h>
#include <arpa/inet.h>
#include <netinet/in.h>
#include <netdb.h>
#include <errno.h>
#include <time.h>
#include <pthread.h>
#include "ockam/transport.h"
#include "syslog.h"

#define MAX_QUEUE_SIZE          512

typedef enum {
    MSG_TERMINATE               = 0x0000,                       // Terminate message loop thread
    MSG_CONNECTION_ACCEPTED     = 0x0001
} SERVER_MESSAGE;

typedef struct {
    SERVER_MESSAGE      message;
    void*               p_context;
} SERVER_DISPATCH;

typedef struct {
    uint16_t                    max_connections;
    uint16_t                    count_connections;
    OCKAM_TRANSPORT_CONNECTION  listener_connction;
    OCKAM_TRANSPORT_CONNECTION  connections[];
} TRANSPORT_SERVER;

#endif //SERVER_TRANSPORT_SERVER_H
