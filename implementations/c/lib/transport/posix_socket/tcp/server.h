/**
 ********************************************************************************************************
 * @file        server.h
 * @brief       Defines server-specific data and functions
 ********************************************************************************************************
 */

#ifndef SERVER_TRANSPORT_SERVER_H
#define SERVER_TRANSPORT_SERVER_H

#include <arpa/inet.h>
#include <errno.h>
#include <netdb.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <ntsid.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <time.h>

#include "ockam/syslog.h"
#include "ockam/transport.h"

#define MAX_QUEUE_SIZE 512

typedef enum {
  kMsgTerminate = 0x0000,  // Terminate message loop thread
  kMsgConnectionAccepted = 0x0001
} SERVER_MESSAGE;

typedef struct {
  SERVER_MESSAGE Message;
  void *Context;
} SERVER_DISPATCH;

typedef struct {
  uint16_t max_connections;
  uint16_t count_connections;
  OckamTransport listener_connction;
  OckamTransport connections[];
} TRANSPORT_SERVER;

#endif  // SERVER_TRANSPORT_SERVER_H
