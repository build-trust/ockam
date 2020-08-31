#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <ctype.h>
#include <fcntl.h>

#include "ockam/log.h"
#include "ockam/io.h"
#include "ockam/io/impl.h"
#include "ockam/transport.h"
#include "ockam/transport/impl.h"
#include "socket.h"
#include "ockam/memory.h"
#include "socket_udp.h"

extern ockam_memory_t* gp_ockam_transport_memory;
ockam_error_t          socket_udp_connect(
           void* ctx, ockam_reader_t** pp_reader, ockam_writer_t** pp_writer, int16_t retry_count, uint16_t retry_interval);
ockam_error_t socket_udp_accept(void*               ctx,
                                ockam_reader_t**    pp_reader,
                                ockam_writer_t**    pp_writer,
                                ockam_ip_address_t* p_remote_address);

ockam_error_t socket_get_local_address(void*, codec_address_t*);
ockam_error_t socket_get_remote_address(void* ctx, codec_address_t*);
ockam_error_t socket_udp_deinit(ockam_transport_t* p_transport);

ockam_transport_vtable_t socket_udp_vtable = {
  socket_udp_connect, socket_udp_accept, socket_get_local_address, socket_get_remote_address, socket_udp_deinit
};

ockam_error_t socket_udp_read(void*, uint8_t*, size_t, size_t*);
ockam_error_t socket_udp_write(void*, uint8_t*, size_t);

ockam_error_t ockam_transport_socket_udp_init(ockam_transport_t*                   p_transport,
                                              ockam_transport_socket_attributes_t* attributes)
{
  ockam_error_t     error    = ockam_transport_posix_socket_error_none;
  socket_udp_ctx_t* p_ctx    = NULL;
  posix_socket_t*   p_socket = NULL;

  p_transport->vtable = &socket_udp_vtable;

  /*
   * Failure to provide a memory allocator is unrecoverable
   */
  if (NULL == attributes->p_memory) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_BAD_PARAMETER;
    goto exit;
  }
  gp_ockam_transport_memory = attributes->p_memory;

  if ((attributes->local_address.ip_address[0] == 0) || (attributes->local_address.port == 0)) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_BAD_PARAMETER;
    goto exit;
  }

  /*
   * set up type-specific storage for this transport instance
   */
  error = ockam_memory_alloc_zeroed(gp_ockam_transport_memory, (void**) &p_ctx, sizeof(socket_udp_ctx_t));
  if (ockam_error_has_error(&error)) goto exit;

  p_socket = &p_ctx->posix_socket;

  int* p_socket_fd = &p_socket->socket_fd;
  *p_socket_fd     = socket(AF_INET, SOCK_DGRAM, 0);
  if (-1 == *p_socket_fd) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_SOCKET_CREATE;
    goto exit;
  }
  if (-1 == fcntl(*p_socket_fd, F_SETFL, O_NONBLOCK)) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_SOCKET_CREATE;
    goto exit;
  }

  if (setsockopt(*p_socket_fd, SOL_SOCKET, SO_KEEPALIVE, &(int) { 1 }, sizeof(int)) < 0) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_CONNECT;
    goto exit;
  }
  if (setsockopt(*p_socket_fd, SOL_SOCKET, SO_REUSEADDR, &(int) { 1 }, sizeof(int)) < 0) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_CONNECT;
    goto exit;
  }
  if (setsockopt(*p_socket_fd, SOL_SOCKET, SO_REUSEPORT, &(int) { 1 }, sizeof(int)) < 0) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_CONNECT;
    goto exit;
  }

  ockam_memory_copy(
    gp_ockam_transport_memory, &p_socket->local_address, &attributes->local_address, sizeof(p_socket->local_address));
  make_socket_address(p_socket->local_address.ip_address, p_socket->local_address.port, &p_socket->local_sockaddr);
  if (0 != bind(*p_socket_fd, (struct sockaddr*) &p_socket->local_sockaddr, sizeof(struct sockaddr_in))) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_SERVER_INIT;
    goto exit;
  }

  if (attributes->remote_address.port > 0) {
    ockam_memory_copy(gp_ockam_transport_memory,
                      &p_socket->remote_address,
                      &attributes->remote_address,
                      sizeof(p_socket->remote_address));
    make_socket_address(p_socket->remote_address.ip_address, p_socket->remote_address.port, &p_socket->remote_sockaddr);
  }
  p_transport->ctx = p_ctx;

exit:
  if (ockam_error_has_error(&error)) {
    ockam_log_error("%s: %d", error.domain, error.code);
    if (p_ctx) ockam_memory_free(gp_ockam_transport_memory, p_ctx, 0);
  }
  return error;
}

ockam_error_t socket_udp_connect(
  void* ctx, ockam_reader_t** pp_reader, ockam_writer_t** pp_writer, int16_t retry_count, uint16_t retry_interval)
{
  (void) retry_count;
  (void) retry_interval;

  ockam_error_t     error     = ockam_transport_posix_socket_error_none;
  socket_udp_ctx_t* p_udp_ctx = (socket_udp_ctx_t*) ctx;

  if (NULL == p_udp_ctx) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_BAD_PARAMETER;
    goto exit;
  }
  posix_socket_t* p_socket = &p_udp_ctx->posix_socket;

  error = make_socket_reader_writer(p_socket, socket_udp_read, socket_udp_write, pp_reader, pp_writer);
  if (ockam_error_has_error(&error)) goto exit;

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t socket_udp_accept(void*               ctx,
                                ockam_reader_t**    pp_reader,
                                ockam_writer_t**    pp_writer,
                                ockam_ip_address_t* p_remote_address)
{
  (void) p_remote_address;

  ockam_error_t     error     = ockam_transport_posix_socket_error_none;
  socket_udp_ctx_t* p_udp_ctx = (socket_udp_ctx_t*) ctx;

  if (NULL == p_udp_ctx) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_BAD_PARAMETER;
    goto exit;
  }
  posix_socket_t* p_socket = &p_udp_ctx->posix_socket;

  error = make_socket_reader_writer(p_socket, socket_udp_read, socket_udp_write, pp_reader, pp_writer);
  if (ockam_error_has_error(&error)) goto exit;

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t socket_udp_read(void* ctx, uint8_t* buffer, size_t buffer_size, size_t* buffer_length)
{
  ockam_error_t     error      = ockam_transport_posix_socket_error_none;
  socket_udp_ctx_t* p_udp_ctx  = (socket_udp_ctx_t*) ctx;
  posix_socket_t*   p_socket   = &p_udp_ctx->posix_socket;
  ssize_t           bytes_read = 0;
  socklen_t         socklen    = 0;

  if (-1 == p_socket->socket_fd) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_SOCKET;
    goto exit;
  }

  socklen    = sizeof(p_socket->remote_sockaddr);
  bytes_read = recvfrom(
    p_socket->socket_fd, buffer, buffer_size, MSG_DONTWAIT, (struct sockaddr*) &p_socket->remote_sockaddr, &socklen);
  if (0 >= bytes_read) {
    if ((errno == EWOULDBLOCK) || (errno == EAGAIN)) {
      error.domain = OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN;
      error.code = OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA;
    } else {
      error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_RECEIVE;
    }
    goto exit;
  }
  char astring[100];
  inet_ntop(AF_INET, &p_socket->remote_address, astring, 100);
  *buffer_length = bytes_read;

exit:
  if (ockam_error_has_error(&error)
      && !(error.code == OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA
           && OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN == error.domain))
    ockam_log_error("%s: %d", error.domain, error.code);

  return error;
}

ockam_error_t socket_udp_write(void* ctx, uint8_t* buffer, size_t buffer_length)
{
  ockam_error_t     error      = ockam_transport_posix_socket_error_none;
  socket_udp_ctx_t* p_udp_ctx  = (socket_udp_ctx_t*) ctx;
  posix_socket_t*   p_socket   = &p_udp_ctx->posix_socket;
  size_t            bytes_sent = 0;

  if (buffer_length > (SIZE_MAX >> 1u)) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_BAD_PARAMETER;
    goto exit;
  }

  bytes_sent = sendto(p_socket->socket_fd,
                      buffer,
                      buffer_length,
                      0,
                      (struct sockaddr*) &p_socket->remote_sockaddr,
                      sizeof(p_udp_ctx->posix_socket.remote_sockaddr));

  if (bytes_sent != buffer_length) {
    if ((errno == EWOULDBLOCK) || (errno == EAGAIN)) {
      error.domain = OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN;
      error.code = OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA;
    }
    else {
      error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_SEND;
    }
  }
exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t socket_get_local_address(void* ctx, codec_address_t* address)
{
  ockam_error_t     error   = ockam_transport_posix_socket_error_none;
  socket_udp_ctx_t* udp_ctx = (socket_udp_ctx_t*) ctx;
  codec_address_t   addr;
  uint8_t           octets[4] = { 0 };

  if (udp_ctx->posix_socket.local_address.ip_address[0] == 0) {
    error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_BAD_ADDRESS;
    goto exit;
  }

  addr.type = ADDRESS_UDP;
  sscanf((char*) udp_ctx->posix_socket.local_address.ip_address,
         "%c.%c.%c.%c",
         &octets[0],
         &octets[1],
         &octets[2],
         &octets[3]);
  addr.address.socket_address.udp_address.port = udp_ctx->posix_socket.local_address.port;

  memcpy(address, &addr, sizeof(addr));

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t socket_get_remote_address(void* ctx, codec_address_t* address)
{
  ockam_error_t error = ockam_transport_posix_socket_error_none;

  error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_BAD_ADDRESS;

  return error;
}

ockam_error_t socket_udp_deinit(ockam_transport_t* p_transport)
{
  socket_udp_ctx_t* p_udp_ctx = (socket_udp_ctx_t*) p_transport->ctx;

  if (p_udp_ctx != NULL) {
    // Close the connection
    if (NULL != p_udp_ctx->posix_socket.p_reader)
      ockam_memory_free(gp_ockam_transport_memory, p_udp_ctx->posix_socket.p_reader, 0);
    if (NULL != p_udp_ctx->posix_socket.p_writer)
      ockam_memory_free(gp_ockam_transport_memory, p_udp_ctx->posix_socket.p_writer, 0);
    ockam_memory_free(gp_ockam_transport_memory, p_udp_ctx, 0);
  }

  return ockam_transport_posix_socket_error_none;
}
