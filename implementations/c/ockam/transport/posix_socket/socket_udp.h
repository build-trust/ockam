#ifndef socket_udp_h
#define socket_udp_h

#include "ockam/transport.h"
#include "ockam/transport/socket.h"

ockam_error_t ockam_transport_socket_udp_init(ockam_transport_t*                   p_transport,
                                              ockam_transport_socket_attributes_t* p_cfg);

// TODO: add ockam_ prefix to types declared here. Review which of them indeed need to be public.

typedef struct socket_udp_ctx {
  posix_socket_t posix_socket;
} socket_udp_ctx_t;

#endif
