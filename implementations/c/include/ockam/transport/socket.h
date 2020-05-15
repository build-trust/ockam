#ifndef SOCKET_H
#define SOCKET_H

#include "ockam/transport.h"
/*
 * tcp socket specific transport
 */
typedef struct ockam_transport_tcp_socket_attributes_t {
  ockam_ip_address_t listen_address;
} ockam_transport_tcp_socket_attributes_t;

OckamError ockam_transport_socket_tcp_init(ockam_transport_t*                       transport,
                                           ockam_transport_tcp_socket_attributes_t* attrs);

#endif
