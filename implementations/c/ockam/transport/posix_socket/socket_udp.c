#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <ctype.h>
#include <unistd.h>

#include "ockam/log/syslog.h"
#include "ockam/io.h"
#include "ockam/io/impl.h"
#include "ockam/transport.h"
#include "ockam/transport/impl.h"
#include "socket.h"
#include "ockam/memory.h"
#include "socket_udp.h"

extern ockam_memory_t* gp_ockam_transport_memory;
ockam_error_t          socket_udp_connect(void*               ctx,
                                          ockam_reader_t**    pp_reader,
                                          ockam_writer_t**    pp_writer,
                                          ockam_ip_address_t* remote_address,
                                          int16_t             retry_count,
                                          uint16_t            retry_interval);
ockam_error_t          socket_udp_accept(void*               ctx,
                                         ockam_reader_t**    pp_reader,
                                         ockam_writer_t**    pp_writer,
                                         ockam_ip_address_t* p_remote_address);
ockam_error_t          socket_udp_deinit(ockam_transport_t* p_transport);

ockam_transport_vtable_t socket_udp_vtable = { socket_udp_connect, socket_udp_accept, socket_udp_deinit };

ockam_error_t socket_udp_read(void*, uint8_t*, size_t, size_t*);
ockam_error_t socket_udp_write(void*, uint8_t*, size_t);

ockam_error_t ockam_transport_socket_udp_init(ockam_transport_t*                   p_transport,
                                              ockam_transport_socket_attributes_t* p_cfg)
{
  ockam_error_t     error    = OCKAM_ERROR_NONE;
  socket_udp_ctx_t* p_ctx    = NULL;
  posix_socket_t*   p_socket = NULL;

  p_transport->vtable = &socket_udp_vtable;

  /*
   * Failure to provide a memory allocator is unrecoverable
   */
  if (NULL == p_cfg->p_memory) {
    error = TRANSPORT_ERROR_BAD_PARAMETER;
    goto exit;
  }
  gp_ockam_transport_memory = p_cfg->p_memory;

  /*
   * set up type-specific storage for this transport instance
   */
  error = ockam_memory_alloc_zeroed(gp_ockam_transport_memory, (void**) &p_ctx, sizeof(socket_udp_ctx_t));
  if (error) goto exit;

  p_socket = &p_ctx->posix_socket;

  int* p_socket_fd = &p_socket->socket_fd;
  *p_socket_fd = socket(AF_INET, SOCK_DGRAM, 0);
  if (-1 == *p_socket_fd) {
    error = TRANSPORT_ERROR_SOCKET_CREATE;
    goto exit;
  }
  if (setsockopt(*p_socket_fd, SOL_SOCKET, SO_KEEPALIVE, &(int) { 1 }, sizeof(int)) < 0) {
    error = TRANSPORT_ERROR_CONNECT;
    goto exit;
  }
  if (setsockopt(*p_socket_fd, SOL_SOCKET, SO_REUSEADDR, &(int) { 1 }, sizeof(int)) < 0) {
    error = TRANSPORT_ERROR_CONNECT;
    goto exit;
  }
  if (setsockopt(*p_socket_fd, SOL_SOCKET, SO_REUSEPORT, &(int) { 1 }, sizeof(int)) < 0) {
    error = TRANSPORT_ERROR_CONNECT;
    goto exit;
  }

  ockam_memory_copy(
    gp_ockam_transport_memory, &p_socket->local_address, &p_cfg->listen_address, sizeof(p_socket->local_address));

  make_socket_address(p_socket->local_address.ip_address, p_socket->local_address.port, &p_socket->local_sockaddr);
  if (0 != bind(*p_socket_fd, (struct sockaddr*) &p_socket->local_sockaddr, sizeof(struct sockaddr_in))) {
    error = TRANSPORT_ERROR_SERVER_INIT;
    goto exit;
  }

  p_transport->ctx = p_ctx;

exit:
  if (error) {
    log_error(error, __func__);
    if (p_ctx) ockam_memory_free(gp_ockam_transport_memory, p_ctx, 0);
  }
  return error;
}

ockam_error_t socket_udp_connect(void*               ctx,
                                 ockam_reader_t**    pp_reader,
                                 ockam_writer_t**    pp_writer,
                                 ockam_ip_address_t* remote_address,
                                 int16_t             retry_count,
                                 uint16_t            retry_interval)
{
  (void) retry_count;
  (void) retry_interval;

  ockam_error_t     error     = OCKAM_ERROR_NONE;
  socket_udp_ctx_t* p_udp_ctx = (socket_udp_ctx_t*) ctx;

  if (NULL == p_udp_ctx) {
    error = TRANSPORT_ERROR_BAD_PARAMETER;
    goto exit;
  }
  posix_socket_t* p_socket = &p_udp_ctx->posix_socket;

  error = make_socket_reader_writer(p_socket, socket_udp_read, socket_udp_write, pp_reader, pp_writer);
  if (error) goto exit;

  ockam_memory_copy(gp_ockam_transport_memory, &p_socket->remote_address, remote_address, sizeof(*remote_address));

  error = make_socket_address(
    remote_address->ip_address, remote_address->port, (struct sockaddr_in*) &p_socket->remote_sockaddr);
  if (error) goto exit;

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t
socket_udp_accept(void* ctx, ockam_reader_t** pp_reader, ockam_writer_t** pp_writer, ockam_ip_address_t* p_remote_address)
{
  (void) p_remote_address;

  ockam_error_t     error     = OCKAM_ERROR_NONE;
  socket_udp_ctx_t* p_udp_ctx = (socket_udp_ctx_t*) ctx;

  if (NULL == p_udp_ctx) {
    error = TRANSPORT_ERROR_BAD_PARAMETER;
    goto exit;
  }
  posix_socket_t* p_socket = &p_udp_ctx->posix_socket;

  error = make_socket_reader_writer(p_socket, socket_udp_read, socket_udp_write, pp_reader, pp_writer);
  if (error) goto exit;

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t socket_udp_read(void* ctx, uint8_t* buffer, size_t buffer_size, size_t* buffer_length)
{
  ockam_error_t     error      = OCKAM_ERROR_NONE;
  socket_udp_ctx_t* p_udp_ctx  = (socket_udp_ctx_t*) ctx;
  posix_socket_t*   p_socket   = &p_udp_ctx->posix_socket;
  ssize_t           bytes_read = 0;
  socklen_t         socklen    = 0;

  if (-1 == p_socket->socket_fd) {
    error = TRANSPORT_ERROR_SOCKET;
    goto exit;
  }

  socklen    = sizeof(p_socket->remote_sockaddr);
  bytes_read = recvfrom(
    p_socket->socket_fd, buffer, buffer_size, MSG_WAITALL, (struct sockaddr*) &p_socket->remote_sockaddr, &socklen);
  if (0 == bytes_read) {
    error = TRANSPORT_ERROR_RECEIVE;
    goto exit;
  }
  char astring[100];
  inet_ntop(AF_INET, &p_socket->remote_address, astring, 100);
  *buffer_length = bytes_read;

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t socket_udp_write(void* ctx, uint8_t* buffer, size_t buffer_length)
{
  ockam_error_t     error       = OCKAM_ERROR_NONE;
  socket_udp_ctx_t* p_udp_ctx   = (socket_udp_ctx_t*) ctx;
  posix_socket_t*   p_socket    = &p_udp_ctx->posix_socket;
  size_t            bytes_sent  = 0;

  bytes_sent = sendto(p_socket->socket_fd,
                      buffer,
                      buffer_length,
                      0,
                      (struct sockaddr*) &p_socket->remote_sockaddr,
                      sizeof(p_udp_ctx->posix_socket.remote_sockaddr));
  if (bytes_sent != buffer_length) {
    error = TRANSPORT_ERROR_SEND;
    goto exit;
  }

exit:
  if (error) log_error(error, __func__);
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

  return 0;
}
