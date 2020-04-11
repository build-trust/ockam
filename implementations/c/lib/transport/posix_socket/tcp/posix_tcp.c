/**
 ********************************************************************************************************
 * @file        connection.h
 * @brief       Defines the different connection types.
 ********************************************************************************************************
 */
/*
 ********************************************************************************************************
 *                                             INCLUDE FILES *
 ********************************************************************************************************
 */

#include "posix_tcp.h"

#include <unistd.h>

#include "../posix_socket.h"
#include "ockam/syslog.h"
#include "ockam/transport.h"
//!!
#include <stdio.h>
#include <errno.h>

/*
 ********************************************************************************************************
 *                               Forward function prototype declarations *
 ********************************************************************************************************
 */
TransportError PosixTcpInitialize(OckamTransportCtx *, OckamTransportConfig *);
TransportError PosixTcpListenBlocking(Connection *, OckamInternetAddress *, OckamTransportCtx *);
TransportError PosixTcpConnectBlocking(Connection *, OckamInternetAddress *);
TransportError PosixTcpReceiveBlocking(Connection *, void *, uint16_t, uint16_t *);
TransportError PosixTcpSendBlocking(Connection *, void *, uint16_t);
TransportError PosixTcpUninitialize(Connection *connection);

/*
 ********************************************************************************************************
 *                                        Global Variables *
 ********************************************************************************************************
 */

/**
 * This is the vtable for the posix tcp transport
 */
OckamTransport ockamPosixTcpTransport = {
    (TransportError(*)(OckamTransportCtx *, OckamTransportConfig *)) & PosixTcpInitialize,

    (TransportError(*)(OckamTransportCtx, OckamInternetAddress *, OckamTransportCtx *)) & PosixTcpListenBlocking,

    (TransportError(*)(OckamTransportCtx, OckamInternetAddress *)) & PosixTcpConnectBlocking,

    (TransportError(*)(OckamTransportCtx, void *, uint16_t, uint16_t *)) & PosixTcpReceiveBlocking,

    (TransportError(*)(OckamTransportCtx, void *, uint16_t)) & PosixTcpSendBlocking,

    (TransportError(*)(OckamTransportCtx)) & PosixTcpUninitialize};

/*
 ********************************************************************************************************
 *                                         Functions *
 ********************************************************************************************************
 */

TransportError PosixTcpInitialize(OckamTransportCtx *connection, OckamTransportConfig *config) {
  TransportError status = kErrorNone;

  // Allocate the memory, zero it out, and set the pointer to the interface
  *connection = (Connection *)malloc(sizeof(Connection));
  if (NULL == *connection) {
    status = kMalloc;
    log_error(status, "malloc failed in ockam_init_posix_tcp_transport");
    goto exit_block;
  }
  memset(*connection, 0, sizeof(Connection));

exit_block:
  if (kErrorNone != status) {
    if (NULL != *connection) free(*connection);
  }
  return status;
}
TransportError PosixTcpListenBlocking(Connection *listener, OckamInternetAddress *address,
                                      OckamTransportCtx *newTransportInstance) {
  OckamTransportCtx new_connection = NULL;
  TransportError status = kErrorNone;
  PosixSocket *listen_socket = &listener->type.posixSocket;
  PosixSocket *accept_socket = NULL;
  in_port_t port = DEFAULT_TCP_LISTEN_PORT;
  OckamTransportConfig tcpConfig = {kBlocking};
  char *local_ip_address = NULL;

  // Create the socket
  listen_socket->socket = socket(AF_INET, SOCK_STREAM, 0);
  if (-1 == listen_socket->socket) {
    status = kServerInit;
    log_error(status, "failed to create listen socket in PosixTcpListenBlocking");
    goto exit_block;
  }
  if (setsockopt(listen_socket->socket, SOL_SOCKET, SO_REUSEADDR, &(int){1}, sizeof(int)) < 0) {
    status = kServerInit;
    log_error(status, "failed setsockopt in PosixTcpListenBlocking");
    goto exit_block;
  }
  if (setsockopt(listen_socket->socket, SOL_SOCKET, SO_REUSEPORT, &(int){1}, sizeof(int)) < 0) {
    status = kServerInit;
    log_error(status, "failed setsockopt in PosixTcpListenBlocking");
    goto exit_block;
  }
  if (setsockopt(listen_socket->socket, SOL_SOCKET, SO_KEEPALIVE, &(int){1}, sizeof(int)) < 0) {
    status = kServerInit;
    log_error(status, "failed setsockopt in PosixTcpListenBlocking");
    goto exit_block;
  }

  // Save IP address and port and construct address, if provided
  if (NULL != address) {
    memcpy(&listen_socket->localAddress, address, sizeof(listen_socket->localAddress));
    local_ip_address = address->IPAddress;
    port = address->port;
  }

  // Construct the address
  status = MakeSocketAddress(local_ip_address, port, &listen_socket->socketAddress);
  if (kErrorNone != status) {
    log_error(status, "local IP address invalid in PosixTcpListenBlocking ");
    goto exit_block;
  }

  // Bind the address to the socket
  if (0 != bind(listen_socket->socket, (struct sockaddr *)&listen_socket->socketAddress,
                sizeof(listen_socket->socketAddress))) {
    status = kReceive;
    log_error(status, "bind failed in PosixTcpListenBlocking");
    goto exit_block;
  }

  // Initialize the new connection
  status = PosixTcpInitialize(&new_connection, &tcpConfig);
  if (kErrorNone != status) {
    log_error(status, "failed to create new connection in PosixTcpListenBlocking");
    goto exit_block;
  }
  accept_socket = &((Connection *)new_connection)->type.posixSocket;

  // Listen
  if (0 != listen(listen_socket->socket,
                  1)) {  // #revisit when multiple connections implemented
    status = kServerInit;
    log_error(status, "Listen failed");
    goto exit_block;
  }

  // Wait for the connection
  accept_socket->socket = accept(listen_socket->socket, NULL, 0);
  if (-1 == accept_socket->socket) {
    status = kAcceptConnection;
    log_error(status, "accept failed");
    goto exit_block;
  }
  accept_socket->isConnected = 1;

  // It all worked. Copy the new connection to the caller's variable.
  *newTransportInstance = new_connection;

exit_block:
  if (kErrorNone != status) {
    if (-1 != listen_socket->socket) close(listen_socket->socket);
    if (NULL != new_connection) PosixTcpUninitialize(new_connection);
  }
  return status;
}

TransportError PosixTcpConnectBlocking(Connection *connection, OckamInternetAddress *address) {
  TransportError status = kErrorNone;
  PosixSocket *posix_socket = &connection->type.posixSocket;

  // Save the host IP address and port
  memcpy(&posix_socket->remoteAddress, address, sizeof(*address));

  // Construct the server address for connection
  status = MakeSocketAddress(address->IPAddress, address->port, &posix_socket->socketAddress);
  if (kErrorNone != status) {
    status = kBadParameter;
    log_error(status, "MakeSocketAddress failed in PosixTcpConnectBlocking");
  }

  // Create the socket
  posix_socket->socket = socket(AF_INET, SOCK_STREAM, 0);
  if (-1 == posix_socket->socket) {
    status = kCreateSocket;
    log_error(status, "socket failed in p_socket");
    goto exit_block;
  }
  if (setsockopt(posix_socket->socket, SOL_SOCKET, SO_REUSEADDR, &(int){1}, sizeof(int)) < 0) {
    status = kServerInit;
    log_error(status, "failed setsockopt in PosixTcpListenBlocking");
    goto exit_block;
  }
  if (setsockopt(posix_socket->socket, SOL_SOCKET, SO_REUSEPORT, &(int){1}, sizeof(int)) < 0) {
    status = kServerInit;
    log_error(status, "failed setsockopt in PosixTcpListenBlocking");
    goto exit_block;
  }
  if (setsockopt(posix_socket->socket, SOL_SOCKET, SO_KEEPALIVE, &(int){1}, sizeof(int)) < 0) {
    status = kServerInit;
    log_error(status, "failed setsockopt in PosixTcpListenBlocking");
    goto exit_block;
  }

  // Try to connect
  if (connect(posix_socket->socket, (struct sockaddr *)&posix_socket->socketAddress,
              sizeof(posix_socket->socketAddress)) < 0) {
    status = kConnect;
    log_error(status, "connect failed in PosixTcpConnectBlocking");
    goto exit_block;
  }
  posix_socket->isConnected = 1;

exit_block:
  return status;
}

TransportError PosixTcpReceiveBlocking(Connection *connection, void *buffer, uint16_t size, uint16_t *bytesReceived) {
  TransportError status = kErrorNone;
  POSIX_TCP_SOCKET *p_tcp = NULL;
  Transmission *p_transmission = NULL;
  ssize_t bytes_read = 0;

  if (NULL == connection) {
    status = kBadParameter;
    log_error(status, "connection must not be NULL in PosixTcpReceiveBlocking");
  }

  p_tcp = &connection->type.posixTcpSocket;

  if (1 != p_tcp->posixSocket.isConnected) {
    status = kNotConnected;
    log_error(status, "tcp socket must be connected for read operation");
    goto exit_block;
  }
  p_transmission = &p_tcp->posixSocket.receiveTransmission;

  // See if this is a continuation or a new transmission
  if (p_transmission->status != kMoreData) {
    memset(p_transmission, 0, sizeof(*p_transmission));
  }
  p_transmission->buffer = buffer;
  p_transmission->buffer_size = size;
  p_transmission->buffer_remaining = size;

  if (kMoreData != p_transmission->status) {
    uint16_t recv_len = 0;
    bytes_read = recv(p_tcp->posixSocket.socket, &recv_len, sizeof(uint16_t), 0);
    // Must convert from network order to native endianness
    p_transmission->transmit_length = ntohs(recv_len);
    if (-1 == bytes_read) {
      status = kReceive;
      goto exit_block;
    }
    if (p_transmission->transmit_length > 0) p_transmission->status = kMoreData;
  }

  bytes_read = 0;
  while ((kMoreData == p_transmission->status) && (p_transmission->buffer_remaining > 0)) {
    uint16_t bytes_to_read = 0;
    ssize_t recv_status = 0;
    bytes_to_read = p_transmission->transmit_length - p_transmission->bytes_transmitted;
    if (bytes_to_read > p_transmission->buffer_remaining) bytes_to_read = p_transmission->buffer_remaining;
    recv_status = recv(p_tcp->posixSocket.socket, p_transmission->buffer + bytes_read, bytes_to_read, 0);
    if (-1 == recv_status) {
      status = kReceive;
      log_error(status, (char *)__FUNCTION__);
      goto exit_block;
    }
    bytes_read += recv_status;

    p_transmission->bytes_transmitted += recv_status;
    p_transmission->buffer_remaining -= recv_status;
    if (p_transmission->bytes_transmitted < p_transmission->transmit_length)
      p_transmission->status = kMoreData;
    else
      p_transmission->status = kErrorNone;
  }
  *bytesReceived = bytes_read;
  status = p_transmission->status;
  if (status == kErrorNone) memset(p_transmission, 0, sizeof(*p_tcp));

exit_block:
  return status;
}

TransportError PosixTcpSendBlocking(Connection *connection, void *buffer, uint16_t length) {
  TransportError status = kErrorNone;
  POSIX_TCP_SOCKET *p_tcp = NULL;
  Transmission *transmission;
  ssize_t bytes_sent = 0;

  if (NULL == connection) {
    status = kBadParameter;
    log_error(status, "transport must not be NULL in PosixTcpSendBlocking");
  }

  p_tcp = &connection->type.posixTcpSocket;
  transmission = &p_tcp->posixSocket.receiveTransmission;

  if (1 != p_tcp->posixSocket.isConnected) {
    status = kNotConnected;
    log_error(status, "tcp socket must be connected for write operation");
    goto exit_block;
  }

  // Convert from native endianness to network order
  uint16_t send_len = htons(length);
  bytes_sent = send(p_tcp->posixSocket.socket, (void *)&send_len, sizeof(uint16_t), 0);
  if (sizeof(uint16_t) != bytes_sent) {
    status = kSend;
    goto exit_block;
  }

  bytes_sent = send(p_tcp->posixSocket.socket, buffer, length, 0);
  if (bytes_sent != length) {
    status = kSend;
    goto exit_block;
  }

exit_block:
  return status;
}

TransportError PosixTcpUninitialize(Connection *connection) {
  TransportError status = kErrorNone;
  PosixSocket *p_socket = NULL;

  if (NULL == connection) {
    status = kBadParameter;
    log_error(status, "connection must not be NULL in PosixTcpUninitialize");
    goto exit_block;
  }

  p_socket = &connection->type.posixSocket;

  // Close socket and free memory
  if (p_socket->socket > 0) close(p_socket->socket);

  free(connection);

exit_block:
  return status;
}
