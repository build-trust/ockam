#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <ctype.h>
#include <unistd.h>

#include "ockam/log.h"
#include "ockam/io.h"
#include "ockam/io/impl.h"
#include "ockam/transport.h"
#include "ockam/transport/impl.h"
#include "socket.h"
#include "socket_tcp.h"
#include "ockam/memory.h"

extern ockam_memory_t* gp_ockam_transport_memory;

ockam_transport_vtable_t socket_tcp_vtable = { socket_tcp_connect, socket_tcp_accept, socket_tcp_deinit };

ockam_error_t socket_tcp_read(void*, uint8_t*, size_t, size_t*);
ockam_error_t socket_tcp_write(void*, uint8_t*, size_t);

ockam_error_t ockam_transport_socket_tcp_init(ockam_transport_t* p_transport, ockam_transport_socket_attributes_t* cfg)
{
  ockam_error_t     error = ockam_transport_posix_socket_error_none;
  socket_tcp_ctx_t* p_ctx = NULL;

  p_transport->vtable = &socket_tcp_vtable;

  /*
   * Failure to provide a memory allocator is unrecoverable
   */
  if (NULL == cfg->p_memory) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_BAD_PARAMETER;
    goto exit;
  }
  gp_ockam_transport_memory = cfg->p_memory;

  /*
   * set up type-specific storage for this transport instance
   */
  error = ockam_memory_alloc_zeroed(gp_ockam_transport_memory, (void**) &p_ctx, sizeof(socket_tcp_ctx_t));
  if (ockam_error_has_error(&error)) goto exit;

  p_transport->ctx = p_ctx;
  ockam_memory_copy(
    gp_ockam_transport_memory, &p_ctx->listen_address, &cfg->listen_address, sizeof(ockam_ip_address_t));

exit:
  if (ockam_error_has_error(&error)) {
    ockam_log_error("%s: %d", error.domain, error.code);
    if (p_ctx) ockam_memory_free(gp_ockam_transport_memory, p_ctx, 0);
  }
  return error;
}

ockam_error_t socket_tcp_connect(void*               ctx,
                                 ockam_reader_t**    pp_reader,
                                 ockam_writer_t**    pp_writer,
                                 ockam_ip_address_t* remote_address,
                                 int16_t             retry_count,
                                 uint16_t            retry_interval)
{
  ockam_error_t      error = ockam_transport_posix_socket_error_none;
  struct sockaddr_in socket_address;
  socket_tcp_ctx_t*  p_transport_ctx = (socket_tcp_ctx_t*) ctx;
  tcp_socket_t*      p_tcp_socket    = NULL;
  posix_socket_t*    p_posix_socket  = NULL;

  if (NULL == p_transport_ctx) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_BAD_PARAMETER;
    goto exit;
  }

  error = ockam_memory_alloc_zeroed(gp_ockam_transport_memory, (void**) &p_tcp_socket, sizeof(*p_tcp_socket));
  if (ockam_error_has_error(&error)) goto exit;

  p_posix_socket            = &p_tcp_socket->posix_socket;
  p_posix_socket->socket_fd = -1;
  p_transport_ctx->p_socket = p_tcp_socket;

  error = make_socket_reader_writer(p_posix_socket, socket_tcp_read, socket_tcp_write, pp_reader, pp_writer);
  if (ockam_error_has_error(&error)) goto exit;

  p_transport_ctx->p_socket = p_tcp_socket;

  ockam_memory_copy(
    gp_ockam_transport_memory, &p_posix_socket->remote_address, remote_address, sizeof(*remote_address));

  error = make_socket_address(remote_address->ip_address, remote_address->port, &socket_address);
  if (ockam_error_has_error(&error)) goto exit;

  int attempts       = 0;
  int connect_status = 0;
  do {
    p_posix_socket->socket_fd = socket(AF_INET, SOCK_STREAM, 0);
    if (-1 == p_posix_socket->socket_fd) {
      error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_SOCKET_CREATE;
      goto exit;
    }

    if (setsockopt(p_posix_socket->socket_fd, SOL_SOCKET, SO_KEEPALIVE, &(int) { 1 }, sizeof(int)) < 0) {
      error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_CONNECT;
      goto exit;
    }
    if (setsockopt(p_posix_socket->socket_fd, SOL_SOCKET, SO_REUSEADDR, &(int) { 1 }, sizeof(int)) < 0) {
      error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_CONNECT;
      goto exit;
    }
    if (setsockopt(p_posix_socket->socket_fd, SOL_SOCKET, SO_REUSEPORT, &(int) { 1 }, sizeof(int)) < 0) {
      error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_CONNECT;
      goto exit;
    }
    connect_status = connect(p_posix_socket->socket_fd, (struct sockaddr*) &socket_address, sizeof(socket_address));
    if (connect_status) {
      close(p_posix_socket->socket_fd);
      attempts++;
      if (attempts <= retry_count) { sleep(retry_interval); }
    }
  } while (connect_status && (attempts <= retry_count));
  if (connect_status) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_CONNECT;
    goto exit;
  }

exit:
  if (ockam_error_has_error(&error)) {
    ockam_log_error("%s: %d", error.domain, error.code);
    if (p_tcp_socket) {
      ockam_memory_free(gp_ockam_transport_memory, p_tcp_socket, 0);
      if (p_transport_ctx) p_transport_ctx->p_socket = NULL;
    }
  }
  return error;
}

ockam_error_t
socket_tcp_accept(void* ctx, ockam_reader_t** pp_reader, ockam_writer_t** pp_writer, ockam_ip_address_t* remote_address)
{
  ockam_error_t     error            = ockam_transport_posix_socket_error_none;
  socket_tcp_ctx_t* p_tcp_ctx        = (socket_tcp_ctx_t*) ctx;
  tcp_socket_t*     p_listen_socket  = NULL;
  tcp_socket_t*     p_connect_socket = NULL;

  if (NULL == p_tcp_ctx) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_ACCEPT;
    goto exit;
  }

  error = ockam_memory_alloc_zeroed(gp_ockam_transport_memory, (void**) &p_listen_socket, sizeof(tcp_socket_t));
  if (ockam_error_has_error(&error)) goto exit;
  error = ockam_memory_alloc_zeroed(gp_ockam_transport_memory, (void**) &p_connect_socket, sizeof(tcp_socket_t));
  if (ockam_error_has_error(&error)) goto exit;

  p_tcp_ctx->p_listen_socket = p_listen_socket;
  p_tcp_ctx->p_socket        = p_connect_socket;

  error =
    make_socket_reader_writer(&p_connect_socket->posix_socket, socket_tcp_read, socket_tcp_write, pp_reader, pp_writer);
  if (ockam_error_has_error(&error)) goto exit;

  p_listen_socket->posix_socket.socket_fd = socket(AF_INET, SOCK_STREAM, 0);
  if (-1 == p_listen_socket->posix_socket.socket_fd) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_SERVER_INIT;
    goto exit;
  }

  if (setsockopt(p_listen_socket->posix_socket.socket_fd, SOL_SOCKET, SO_KEEPALIVE, &(int) { 1 }, sizeof(int)) < 0) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_CONNECT;
    goto exit;
  }
  if (setsockopt(p_listen_socket->posix_socket.socket_fd, SOL_SOCKET, SO_REUSEADDR, &(int) { 1 }, sizeof(int)) < 0) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_CONNECT;
    goto exit;
  }
  if (setsockopt(p_listen_socket->posix_socket.socket_fd, SOL_SOCKET, SO_REUSEPORT, &(int) { 1 }, sizeof(int)) < 0) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_CONNECT;
    goto exit;
  }

  if (strlen((char*) p_tcp_ctx->listen_address.ip_address)) {
    ockam_memory_copy(gp_ockam_transport_memory,
                      &p_listen_socket->posix_socket.local_address.ip_address,
                      p_tcp_ctx->listen_address.ip_address,
                      MAX_IP_ADDRESS_LENGTH);
  }
  p_listen_socket->posix_socket.local_address.port = p_tcp_ctx->listen_address.port;

  error = make_socket_address(p_tcp_ctx->listen_address.ip_address,
                              p_tcp_ctx->listen_address.port,
                              &p_listen_socket->posix_socket.remote_sockaddr);
  if (ockam_error_has_error(&error)) goto exit;

  if (0 != bind(p_listen_socket->posix_socket.socket_fd,
                (struct sockaddr*) &p_listen_socket->posix_socket.remote_sockaddr,
                sizeof(p_listen_socket->posix_socket.remote_sockaddr))) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_BAD_PARAMETER;
    ockam_log_error("bind failed in PosixTcpListenBlocking: %s: %d", error.domain, error.code);
    goto exit;
  }

  if (0 != listen(p_listen_socket->posix_socket.socket_fd, 1)) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_LISTEN;
    goto exit;
  }

  // Wait for the connection
  p_connect_socket->posix_socket.socket_fd = accept(p_listen_socket->posix_socket.socket_fd, NULL, 0);
  if (-1 == p_listen_socket->posix_socket.socket_fd) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_ACCEPT;
    goto exit;
  }

  if (p_connect_socket->posix_socket.p_reader) p_connect_socket->posix_socket.p_reader->ctx = p_connect_socket;
  if (p_connect_socket->posix_socket.p_writer) p_connect_socket->posix_socket.p_writer->ctx = p_connect_socket;
  if (pp_reader) *pp_reader = p_connect_socket->posix_socket.p_reader;
  if (pp_writer) *pp_writer = p_connect_socket->posix_socket.p_writer;

exit:
  if (ockam_error_has_error(&error)) {
    ockam_log_error("%s: %d", error.domain, error.code);
    if (p_listen_socket) ockam_memory_free(gp_ockam_transport_memory, p_listen_socket, 0);
    if (p_connect_socket) ockam_memory_free(gp_ockam_transport_memory, p_connect_socket, 0);
  }
  return error;
}

ockam_error_t socket_tcp_read(void* ctx, uint8_t* buffer, size_t buffer_size, size_t* buffer_length)
{
  ockam_error_t   error     = ockam_transport_posix_socket_error_none;
  tcp_socket_t*   p_tcp_ctx = (tcp_socket_t*) ctx;
  posix_socket_t* p_socket  = &p_tcp_ctx->posix_socket;

  tcp_transmission_t* p_transmission = NULL;
  ssize_t             bytes_read     = 0;

  if (-1 == p_socket->socket_fd) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_SOCKET;
    goto exit;
  }
  p_transmission = &p_tcp_ctx->read_transmission;

  // See if this is a continuation or a new transmission
  if (p_transmission->status != OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_MORE_DATA) {
    ockam_memory_set(gp_ockam_transport_memory, p_transmission, 0, sizeof(*p_transmission));
  }
  p_transmission->buffer           = buffer;
  p_transmission->buffer_size      = buffer_size;
  p_transmission->buffer_remaining = buffer_size;

  if (OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_MORE_DATA != p_transmission->status) {
    uint16_t recv_len = 0;
    bytes_read        = recv(p_socket->socket_fd, &recv_len, sizeof(uint16_t), 0);
    // Must convert from network order to native endianness
    p_transmission->transmit_length = ntohs(recv_len);
    if (sizeof(uint16_t) != bytes_read) {
      error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_RECEIVE;
      goto exit;
    }
    if (p_transmission->transmit_length > 0) p_transmission->status = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_MORE_DATA;
  }

  bytes_read = 0;
  while ((OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_MORE_DATA == p_transmission->status) && (p_transmission->buffer_remaining > 0)) {
    ssize_t recv_status   = 0;
    size_t  bytes_to_read = p_transmission->transmit_length - p_transmission->bytes_transmitted;
    if (bytes_to_read > p_transmission->buffer_remaining) bytes_to_read = p_transmission->buffer_remaining;
    recv_status = recv(p_socket->socket_fd, p_transmission->buffer + bytes_read, bytes_to_read, 0);
    if (0 > recv_status) {
      ockam_log_error("receive failed: %ll", recv_status);
      goto exit;
    }
    bytes_read += recv_status;

    p_transmission->bytes_transmitted += recv_status;
    p_transmission->buffer_remaining -= recv_status;
    if (p_transmission->bytes_transmitted < p_transmission->transmit_length) {
      p_transmission->status = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_MORE_DATA;
    } else {
      p_transmission->status = (ockam_error_code_transport_posix_socket_t) OCKAM_ERROR_NONE;
    }
  }
  *buffer_length = bytes_read;
  error.code     = p_transmission->status;
  if (p_transmission->status == OCKAM_ERROR_NONE) {
    ockam_memory_set(gp_ockam_transport_memory, p_transmission, 0, sizeof(*p_transmission));
  }

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t socket_tcp_write(void* ctx, uint8_t* buffer, size_t buffer_length)
{
  ockam_error_t error = ockam_transport_posix_socket_error_none;

  if (buffer_length > (SIZE_MAX >> 1u)) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_BAD_PARAMETER;
    goto exit;
  }

  tcp_socket_t*   p_tcp_ctx   = (tcp_socket_t*) ctx;
  posix_socket_t* p_socket    = &p_tcp_ctx->posix_socket;
  uint16_t        send_length = 0;

  // Convert from native endianness to network order
  send_length        = htons(buffer_length);
  ssize_t bytes_sent = send(p_socket->socket_fd, (void*) &send_length, sizeof(uint16_t), 0);
  if (sizeof(uint16_t) != bytes_sent) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_SEND;
    goto exit;
  }

  bytes_sent = send(p_socket->socket_fd, buffer, buffer_length, 0);
  if (bytes_sent < 0 || bytes_sent != buffer_length) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_SEND;
    goto exit;
  }

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t socket_tcp_deinit(ockam_transport_t* p_transport)
{
  socket_tcp_ctx_t* p_tcp_ctx = (socket_tcp_ctx_t*) p_transport->ctx;

  if (p_tcp_ctx->p_socket != NULL) {
    // Close the connection
    if (NULL != p_tcp_ctx->p_socket->posix_socket.p_reader)
      ockam_memory_free(gp_ockam_transport_memory, p_tcp_ctx->p_socket->posix_socket.p_reader, 0);
    if (NULL != p_tcp_ctx->p_socket->posix_socket.p_writer)
      ockam_memory_free(gp_ockam_transport_memory, p_tcp_ctx->p_socket->posix_socket.p_writer, 0);
    if (NULL != p_tcp_ctx->p_socket) ockam_memory_free(gp_ockam_transport_memory, p_tcp_ctx->p_socket, 0);
  }

  return ockam_transport_posix_socket_error_none;
}
