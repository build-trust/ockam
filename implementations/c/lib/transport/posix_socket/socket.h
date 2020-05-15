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
#include "../transport_impl.h"

/**
 * The PosixSocket is the posix socket specific class data for a posix socket
 * connection (TCP or UDP). Note that TCP sockets are further defined by the
 * POSIX_TCP_SOCKET type.
 */
typedef struct posix_socket_t {
  ockam_reader_t*    p_reader;
  ockam_writer_t*    p_writer;
  ockam_ip_address_t local_address;
  ockam_ip_address_t remote_address;
  int                socket_fd;
  struct sockaddr_in socket_address;
} posix_socket_t;

ockam_error_t make_socket_address(uint8_t* p_ip_address, in_port_t port, struct sockaddr_in* p_socket_address);

#endif
