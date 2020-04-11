#include <arpa/inet.h>
#include <errno.h>
#include <netdb.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/types.h>

#include "ockam/error.h"
#include "ockam/transport.h"

/**
 * One Transmission instance is assigned for each read or write
 */
typedef struct {
  void *buffer;                // buffer to transmit (user-allocated)
  uint16_t buffer_size;        // total size of buffer
  uint16_t buffer_remaining;   //
  uint16_t transmit_length;    // total number of bytes transmit
  uint16_t bytes_transmitted;  // number of bytes transmitted (so far)
  TransportError status;       // transmission completion status
} Transmission;

/**
 * The PosixSocket is the posix socket specific class data for a posix socket
 * connection (TCP or UDP). Note that TCP sockets are further defined by the
 * POSIX_TCP_SOCKET type.
 */
typedef struct {
  uint16_t isConnected;                // connection with remote is established
  OckamInternetAddress localAddress;   // human-friendly local address
  OckamInternetAddress remoteAddress;  // human-friendly remote address
  int socket;                          // posix socket identifier
  struct sockaddr_in socketAddress;    // network-friendly socket information
  Transmission receiveTransmission;
  Transmission sendTransmission;
} PosixSocket;

/**
 * POSIX_TCP_SOCKET has TCP-specific data.
 */
typedef struct {
  PosixSocket posixSocket;
  void *listenCtx;
} POSIX_TCP_SOCKET;

/**
 * Connection is the highest-layer of abstraction for all the connections.
 */
typedef struct {
  union {
    PosixSocket posixSocket;
    POSIX_TCP_SOCKET posixTcpSocket;
  } type;
} Connection, *ConnectionPtr;
