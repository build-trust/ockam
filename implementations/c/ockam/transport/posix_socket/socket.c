#include "ockam/log.h"
#include "ockam/io.h"
#include "ockam/io/impl.h"
#include "ockam/transport.h"
#include "socket.h"
#include <stdio.h>

const char* const OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_DOMAIN = "OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_DOMAIN";

const ockam_error_t ockam_transport_posix_socket_error_none = {
  OCKAM_ERROR_NONE,
  OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_DOMAIN
};

extern ockam_memory_t* gp_ockam_transport_memory;

ockam_error_t make_socket_reader_writer(posix_socket_t* p_ctx,
                                        ockam_error_t (*socket_read)(void*, uint8_t*, size_t, size_t*),
                                        ockam_error_t (*socket_write)(void*, uint8_t*, size_t),
                                        ockam_reader_t** pp_reader,
                                        ockam_writer_t** pp_writer)
{
  ockam_error_t error = ockam_transport_posix_socket_error_none;

  if (NULL != pp_reader) {
    error = ockam_memory_alloc_zeroed(gp_ockam_transport_memory, (void**) &p_ctx->p_reader, sizeof(ockam_reader_t));
    if (ockam_error_has_error(&error)) goto exit;

    p_ctx->p_reader->read = socket_read;
    p_ctx->p_reader->ctx  = p_ctx;
    *pp_reader            = p_ctx->p_reader;
  }
  if (NULL != pp_writer) {
    error = ockam_memory_alloc_zeroed(gp_ockam_transport_memory, (void**) &p_ctx->p_writer, sizeof(ockam_writer_t));
    if (ockam_error_has_error(&error)) goto exit;

    p_ctx->p_writer->write = socket_write;
    p_ctx->p_writer->ctx   = p_ctx;
    *pp_writer             = p_ctx->p_writer;
  }
exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

/**
 * make_socket_address - construct network-friendly address from user-friendly
 * address
 * @param p_ip_address - (in) IP address in "nnn.nnn.nnn.nnn" format
 * @param port - port number, must be non-zero
 * @param p_socket_address - (out) address converted
 * @return - OCKAM_NO_ERR on success
 */
ockam_error_t make_socket_address(const uint8_t* p_ip_address, in_port_t port, struct sockaddr_in* p_socket_address)
{
  ockam_error_t error     = ockam_transport_posix_socket_error_none;
  int           in_status = 0;

  // Get the host IP address and port
  p_socket_address->sin_family = AF_INET;
  p_socket_address->sin_port   = htons(port);
  if (NULL != p_ip_address) {
    in_status = inet_pton(AF_INET, (char*) p_ip_address, &p_socket_address->sin_addr.s_addr);
    if (1 != in_status) {
      error.code = OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_BAD_ADDRESS;
      goto exit;
    }
  } else {
    p_socket_address->sin_addr.s_addr = htonl(INADDR_ANY);
  }

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

void dump_socket(posix_socket_t* ps)
{
  char     local_address[128];
  char     remote_address[128];
  uint16_t local_port;
  uint16_t remote_port;

  inet_ntop(AF_INET, &ps->local_sockaddr.sin_addr, local_address, 128);
  local_port = ntohs(ps->local_sockaddr.sin_port);
  printf("local sockaddr:     : %s\n", local_address);
  printf("local port          : %d\n", local_port);

  inet_ntop(AF_INET, &ps->remote_sockaddr.sin_addr, remote_address, 128);
  remote_port = ntohs(ps->remote_sockaddr.sin_port);
  printf("remote sockaddr:     : %s\n", remote_address);
  printf("remote port          : %d\n", remote_port);
}
