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

// TODO: add ockam_ prefix to types declared here. Review which of them indeed need to be public.

extern const char* const OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_DOMAIN;

extern const ockam_error_t ockam_transport_posix_socket_error_none;

typedef enum {
  OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_SOCKET_CREATE    = 1,  /*!< Failed to create socket */
  OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_CONNECT          = 2,  /*!< Failed to connect  */
  OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_SEND             = 3,  /*!< Failed to send data */
  OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_SERVER_INIT      = 4,  /*!< Server initialization failed */
  OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_RECEIVE          = 5,  /*!< Receive buffer failed */
  OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_BAD_ADDRESS      = 6,  /*!< Bad IP address */
  OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_ACCEPT           = 7,  /*!< Socket accept failed  */
  OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_BAD_PARAMETER    = 8, /*!< Bad parameter */
  OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_MORE_DATA        = 9, /*!< More data available on socket */
  OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_LISTEN           = 10,
  OCKAM_TRANSPORT_POSIX_SOCKET_ERROR_SOCKET           = 11,
} ockam_error_code_transport_posix_socket_t;


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
