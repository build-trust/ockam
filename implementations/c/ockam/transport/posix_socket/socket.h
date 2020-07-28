#ifndef SOCKET_H
#define SOCKET_H
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
#include "ockam/transport.h"
#include "ockam/transport/impl.h"
#include "socket.h"

/**
 * The PosixSocket is the posix socket specific class data for a posix socket
 * connection (TCP or UDP). Note that TCP sockets are further defined by the
 * POSIX_TCP_SOCKET type.
 */
typedef struct posix_socket {
  ockam_reader_t*    p_reader;
  ockam_writer_t*    p_writer;
  ockam_ip_address_t local_address;
  ockam_ip_address_t remote_address;
  int                socket_fd;
  struct sockaddr_in remote_sockaddr;
  struct sockaddr_in local_sockaddr;
} posix_socket_t;

ockam_error_t make_socket_reader_writer(posix_socket_t* p_ctx,
                                        ockam_error_t (*socket_read)(void*, uint8_t*, size_t, size_t*),
                                        ockam_error_t (*socket_write)(void*, uint8_t*, size_t),
                                        ockam_reader_t** pp_reader,
                                        ockam_writer_t** pp_writer);

ockam_error_t make_socket_address(const uint8_t* p_ip_address, in_port_t port, struct sockaddr_in* p_socket_address);

void dump_socket(posix_socket_t* ps);
#endif
