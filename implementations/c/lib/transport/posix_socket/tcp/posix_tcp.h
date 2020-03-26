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
  void *buffer;                     // buffer to transmit (user-allocated)
  uint16_t bufferSize;              // number of bytes to transmit (write) or buffer size (read)
  uint16_t bytesTransmitted;        // number of bytes transmitted (so far)
  TransportError completionStatus;  // transmission completion status
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
 * For POSIX_TCP_SOCKETs, each transmission of a user's buffer is preceded by a
 * TCP_METa_PACKET that indicates the total length of the buffer. Since TCP
 * operates on streams, this is necessary to detect when the sent buffer has
 * been completely received. Doing it this way prevents an additional memory
 * allocation and copy for each buffer sent and received.
 */
typedef struct {
  uint16_t this_packet_length;
  uint16_t next_packet_length;
} TCP_META_PACKET;

/**
 * POSIX_TCP_SOCKET has TCP-specific data.
 */
typedef struct {
  PosixSocket posixSocket;
  void *listenCtx;
  TCP_META_PACKET receiveMeta;
  TCP_META_PACKET sendMeta;
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
