#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <ctype.h>
#include "ockam/syslog.h"
#include "ockam/io.h"
#include "ockam/io/io_impl.h"
#include "ockam/transport.h"
#include "../transport_impl.h"
#include "socket.h"
#include "socket_tcp.h"

ockam_transport_vtable_t socket_tcp_vtable = { socket_tcp_connect, socket_tcp_accept, socket_tcp_deinit };

ockam_error_t socket_tcp_read(void*, uint8_t*, size_t, size_t*);
ockam_error_t socket_tcp_write(void*, uint8_t*, size_t);

ockam_error_t make_socket_reader_writer(posix_socket_t* p_ctx, ockam_reader_t** pp_reader, ockam_writer_t** pp_writer)
{
  ockam_error_t error = TRANSPORT_ERROR_NONE;
  if (NULL != pp_reader) {
    p_ctx->p_reader = (ockam_reader_t*) calloc(1, sizeof(ockam_reader_t));
    if (NULL == p_ctx->p_reader) {
      error = TRANSPORT_ERROR_ALLOC;
      goto exit;
    }
    p_ctx->p_reader->read = socket_tcp_read;
    p_ctx->p_reader->ctx  = p_ctx;
    *pp_reader            = p_ctx->p_reader;
  }
  if (NULL != pp_writer) {
    p_ctx->p_writer = (ockam_writer_t*) calloc(1, sizeof(ockam_writer_t));
    if (NULL == p_ctx->p_writer) {
      error = TRANSPORT_ERROR_ALLOC;
      goto exit;
    }
    p_ctx->p_writer->write = socket_tcp_write;
    p_ctx->p_writer->ctx   = p_ctx;
    *pp_writer             = p_ctx->p_writer;
  }
exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t ockam_transport_socket_tcp_init(ockam_transport_t**                      pp_transport,
                                              ockam_transport_tcp_socket_attributes_t* cfg)
{
  ockam_error_t               error       = OCKAM_ERROR_NONE;
  socket_tcp_transport_ctx_t* p_ctx       = NULL;
  ockam_transport_t*          p_transport = NULL;
  *pp_transport                           = NULL;

  p_transport = (ockam_transport_t*) calloc(1, sizeof(ockam_transport_t));
  if (NULL == p_transport) {
    error = TRANSPORT_ERROR_ALLOC;
    goto exit;
  }
  p_transport->vtable = &socket_tcp_vtable;

  /*
   * set up type-specific storage for this transport instance
   */
  p_ctx = (socket_tcp_transport_ctx_t*) calloc(1, sizeof(socket_tcp_transport_ctx_t));
  if (NULL == p_ctx) {
    error = TRANSPORT_ERROR_ALLOC;
    goto exit;
  }
  p_transport->ctx = p_ctx;
  if (cfg) memcpy(&p_ctx->listen_address, &cfg->listen_address, sizeof(ockam_ip_address_t));

  *pp_transport = p_transport;

exit:
  if (error) {
    log_error(error, __func__);
    if (p_transport) free(p_transport);
    if (p_ctx) free(p_ctx);
  }
  return error;
}

ockam_error_t socket_tcp_connect(void*               ctx,
                                 ockam_reader_t**    pp_reader,
                                 ockam_writer_t**    pp_writer,
                                 ockam_ip_address_t* remote_address)
{
  ockam_error_t               error = OCKAM_ERROR_NONE;
  struct sockaddr_in          socket_address;
  socket_tcp_transport_ctx_t* p_transport_ctx = (socket_tcp_transport_ctx_t*) ctx;
  tcp_socket_t*               p_tcp_socket    = (tcp_socket_t*) calloc(1, sizeof(*p_tcp_socket));
  posix_socket_t*             p_posix_socket  = &p_tcp_socket->posix_socket;

  if (NULL == p_transport_ctx || NULL == p_tcp_socket) {
    error = TRANSPORT_ERROR_BAD_PARAMETER;
    goto exit;
  }

  p_posix_socket->socket_fd = -1;
  p_transport_ctx->p_socket = p_tcp_socket;

  error = make_socket_reader_writer(p_posix_socket, pp_reader, pp_writer);
  if (error) goto exit;

  p_transport_ctx->p_socket = p_tcp_socket;

  memcpy(&p_posix_socket->remote_address, remote_address, sizeof(*remote_address));

  error = make_socket_address(remote_address->ip_address, remote_address->port, &socket_address);
  if (error) goto exit;

  p_posix_socket->socket_fd = socket(AF_INET, SOCK_STREAM, 0);
  if (-1 == p_posix_socket->socket_fd) {
    error = TRANSPORT_ERROR_SOCKET_CREATE;
    goto exit;
  }

  if (setsockopt(p_posix_socket->socket_fd, SOL_SOCKET, SO_KEEPALIVE, &(int) { 1 }, sizeof(int)) < 0) {
    error = TRANSPORT_ERROR_CONNECT;
    goto exit;
  }
  if (setsockopt(p_posix_socket->socket_fd, SOL_SOCKET, SO_REUSEADDR, &(int) { 1 }, sizeof(int)) < 0) {
    error = TRANSPORT_ERROR_CONNECT;
    goto exit;
  }
  if (setsockopt(p_posix_socket->socket_fd, SOL_SOCKET, SO_REUSEPORT, &(int) { 1 }, sizeof(int)) < 0) {
    error = TRANSPORT_ERROR_CONNECT;
    goto exit;
  }

  if (connect(p_posix_socket->socket_fd, (struct sockaddr*) &socket_address, sizeof(socket_address)) < 0) {
    error = TRANSPORT_ERROR_CONNECT;
    goto exit;
  }

exit:
  if (error) {
    log_error(error, __func__);
    if (p_tcp_socket) {
      free(p_tcp_socket);
      p_transport_ctx->p_socket = NULL;
    }
  }
  return error;
}

ockam_error_t
socket_tcp_accept(void* ctx, ockam_reader_t** pp_reader, ockam_writer_t** pp_writer, ockam_ip_address_t* remote_address)
{
  ockam_error_t               error            = TRANSPORT_ERROR_NONE;
  socket_tcp_transport_ctx_t* p_tcp_ctx        = (socket_tcp_transport_ctx_t*) ctx;
  tcp_socket_t*               p_listen_socket  = (tcp_socket_t*) calloc(1, sizeof(struct tcp_socket_t));
  tcp_socket_t*               p_connect_socket = (tcp_socket_t*) calloc(1, sizeof(struct tcp_socket_t));

  if (NULL == p_tcp_ctx || NULL == p_listen_socket || NULL == p_connect_socket) {
    error = TRANSPORT_ERROR_ACCEPT;
    goto exit;
  }

  p_tcp_ctx->p_listen_socket = p_listen_socket;
  p_tcp_ctx->p_socket        = p_connect_socket;

  error = make_socket_reader_writer(&p_connect_socket->posix_socket, pp_reader, pp_writer);
  if (error) goto exit;

  p_listen_socket->posix_socket.socket_fd = socket(AF_INET, SOCK_STREAM, 0);
  if (-1 == p_listen_socket->posix_socket.socket_fd) {
    error = TRANSPORT_ERROR_SERVER_INIT;
    goto exit;
  }

  if (setsockopt(p_listen_socket->posix_socket.socket_fd, SOL_SOCKET, SO_KEEPALIVE, &(int) { 1 }, sizeof(int)) < 0) {
    error = TRANSPORT_ERROR_CONNECT;
    goto exit;
  }
  if (setsockopt(p_listen_socket->posix_socket.socket_fd, SOL_SOCKET, SO_REUSEADDR, &(int) { 1 }, sizeof(int)) < 0) {
    error = TRANSPORT_ERROR_CONNECT;
    goto exit;
  }
  if (setsockopt(p_listen_socket->posix_socket.socket_fd, SOL_SOCKET, SO_REUSEPORT, &(int) { 1 }, sizeof(int)) < 0) {
    error = TRANSPORT_ERROR_CONNECT;
    goto exit;
  }

  if (strlen((char*) p_tcp_ctx->listen_address.ip_address)) {
    memcpy(&p_listen_socket->posix_socket.local_address.ip_address,
           p_tcp_ctx->listen_address.ip_address,
           MAX_IP_ADDRESS_LENGTH);
  }
  p_listen_socket->posix_socket.local_address.port = p_tcp_ctx->listen_address.port;

  error = make_socket_address(p_tcp_ctx->listen_address.ip_address,
                              p_tcp_ctx->listen_address.port,
                              &p_listen_socket->posix_socket.socket_address);
  if (error) goto exit;

  if (0 != bind(p_listen_socket->posix_socket.socket_fd,
                (struct sockaddr*) &p_listen_socket->posix_socket.socket_address,
                sizeof(p_listen_socket->posix_socket.socket_address))) {
    error = TRANSPORT_ERROR_BAD_PARAMETER;
    log_error(error, "bind failed in PosixTcpListenBlocking");
    goto exit;
  }

  if (0 != listen(p_listen_socket->posix_socket.socket_fd, 1)) {
    error = TRANSPORT_ERROR_LISTEN;
    goto exit;
  }

  // Wait for the connection
  p_connect_socket->posix_socket.socket_fd = accept(p_listen_socket->posix_socket.socket_fd, NULL, 0);
  if (-1 == p_listen_socket->posix_socket.socket_fd) {
    error = TRANSPORT_ERROR_ACCEPT;
    goto exit;
  }

  if (p_connect_socket->posix_socket.p_reader) p_connect_socket->posix_socket.p_reader->ctx = p_connect_socket;
  if (p_connect_socket->posix_socket.p_writer) p_connect_socket->posix_socket.p_writer->ctx = p_connect_socket;
  if (pp_reader) *pp_reader = p_connect_socket->posix_socket.p_reader;
  if (pp_writer) *pp_writer = p_connect_socket->posix_socket.p_writer;

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t socket_tcp_read(void* ctx, uint8_t* buffer, size_t buffer_size, size_t* buffer_length)
{
  ockam_error_t   error     = OCKAM_ERROR_NONE;
  tcp_socket_t*   p_tcp_ctx = (tcp_socket_t*) ctx;
  posix_socket_t* p_socket  = &p_tcp_ctx->posix_socket;

  tcp_transmission_t* p_transmission = NULL;
  ssize_t             bytes_read     = 0;

  if (-1 == p_socket->socket_fd) {
    error = TRANSPORT_ERROR_SOCKET;
    goto exit;
  }
  p_transmission = &p_tcp_ctx->read_transmission;

  // See if this is a continuation or a new transmission
  if (p_transmission->status != TRANSPORT_ERROR_MORE_DATA) memset(p_transmission, 0, sizeof(*p_transmission));
  p_transmission->buffer           = buffer;
  p_transmission->buffer_size      = buffer_size;
  p_transmission->buffer_remaining = buffer_size;

  if (TRANSPORT_ERROR_MORE_DATA != p_transmission->status) {
    uint16_t recv_len = 0;
    bytes_read        = recv(p_socket->socket_fd, &recv_len, sizeof(uint16_t), 0);
    // Must convert from network order to native endianness
    p_transmission->transmit_length = ntohs(recv_len);
    if (-1 == bytes_read) {
      error = TRANSPORT_ERROR_RECEIVE;
      goto exit;
    }
    if (p_transmission->transmit_length > 0) p_transmission->status = TRANSPORT_ERROR_MORE_DATA;
  }

  bytes_read = 0;
  while ((TRANSPORT_ERROR_MORE_DATA == p_transmission->status) && (p_transmission->buffer_remaining > 0)) {
    uint16_t bytes_to_read = 0;
    ssize_t  recv_status   = 0;
    bytes_to_read          = p_transmission->transmit_length - p_transmission->bytes_transmitted;
    if (bytes_to_read > p_transmission->buffer_remaining) bytes_to_read = p_transmission->buffer_remaining;
    recv_status = recv(p_socket->socket_fd, p_transmission->buffer + bytes_read, bytes_to_read, 0);
    if (-1 == recv_status) {
      log_error(recv_status, "receive failed");
      goto exit;
    }
    bytes_read += recv_status;

    p_transmission->bytes_transmitted += recv_status;
    p_transmission->buffer_remaining -= recv_status;
    if (p_transmission->bytes_transmitted < p_transmission->transmit_length) {
      p_transmission->status = TRANSPORT_ERROR_MORE_DATA;
    }
    else { p_transmission->status = TRANSPORT_ERROR_NONE; }
  }
  *buffer_length = bytes_read;
  error          = p_transmission->status;
  if (error == TRANSPORT_ERROR_NONE) memset(p_transmission, 0, sizeof(*p_transmission));

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t socket_tcp_write(void* ctx, uint8_t* buffer, size_t buffer_length)
{
  ockam_error_t   error       = OCKAM_ERROR_NONE;
  tcp_socket_t*   p_tcp_ctx   = (tcp_socket_t*) ctx;
  posix_socket_t* p_socket    = &p_tcp_ctx->posix_socket;
  uint16_t        send_length = 0;
  size_t          bytes_sent  = 0;

  // Convert from native endianness to network order
  send_length = htons(buffer_length);
  bytes_sent  = send(p_socket->socket_fd, (void*) &send_length, sizeof(uint16_t), 0);
  if (sizeof(uint16_t) != bytes_sent) {
    error = TRANSPORT_ERROR_SEND;
    goto exit;
  }

  bytes_sent = send(p_socket->socket_fd, buffer, buffer_length, 0);
  if (bytes_sent != buffer_length) {
    error = TRANSPORT_ERROR_SEND;
    goto exit;
  }

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t socket_tcp_deinit(ockam_transport_t* p_transport)
{
  socket_tcp_transport_ctx_t* p_transport_ctx = (socket_tcp_transport_ctx_t*) p_transport->ctx;

  if (p_transport_ctx->p_socket != NULL) {
    // Close the connection
    if (NULL != p_transport_ctx->p_socket->posix_socket.p_reader)
      free(p_transport_ctx->p_socket->posix_socket.p_reader);
    if (NULL != p_transport_ctx->p_socket->posix_socket.p_writer)
      free(p_transport_ctx->p_socket->posix_socket.p_writer);
    if (NULL != p_transport_ctx->p_socket) free(p_transport_ctx->p_socket);
  }
  free(p_transport);

  return 0;
}
