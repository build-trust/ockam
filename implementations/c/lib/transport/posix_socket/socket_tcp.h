#ifndef transport_io_impl_h
#define transport_io_impl_h

#include <stdio.h>
#include "ockam/io.h"
#include "../transport_impl.h"

/**
 * One Transmission instance is assigned for each read or write
 */
typedef struct tcp_transmission_t {
  uint8_t*      buffer;      // buffer to transmit (user-allocated)
  size_t        buffer_size; // number of bytes to transmit (write) or buffer size (read)
  size_t        buffer_remaining;
  size_t        transmit_length;
  size_t        bytes_transmitted; // number of bytes transmitted (so far)
  ockam_error_t status;
  ockam_error_t error; // transmission completion status
} tcp_transmission_t;

typedef struct tcp_socket_t {
  posix_socket_t     posix_socket;
  tcp_transmission_t read_transmission;
  tcp_transmission_t write_transmission;
} tcp_socket_t;

typedef struct socket_tcp_transport_ctx_t {
  ockam_ip_address_t listen_address;
  tcp_socket_t*      p_listen_socket;
  tcp_socket_t*      p_socket; // ToDo: make this a linked list
} socket_tcp_transport_ctx_t;

/*
 * ockam_io_t functions
 */

ockam_error_t
              socket_tcp_connect(void* ctx, ockam_reader_t** reader, ockam_writer_t** writer, ockam_ip_address_t* remote_address);
ockam_error_t socket_tcp_accept(void*               ctx,
                                ockam_reader_t**    pp_reader,
                                ockam_writer_t**    pp_writer,
                                ockam_ip_address_t* remote_address);
ockam_error_t socket_tcp_deinit(ockam_transport_t* transport);

#endif