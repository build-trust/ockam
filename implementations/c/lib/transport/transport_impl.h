#ifndef OCKAM_TRANSPORT_IMPL_H
#define OCKAM_TRANSPORT_IMPL_H

typedef struct ockam_transport_vtable_t {
  ockam_error_t (*connect)(void*               ctx,
                           ockam_reader_t**    reader,
                           ockam_writer_t**    writer,
                           ockam_ip_address_t* remote_address);
  ockam_error_t (*accept)(void*               ctx,
                          ockam_reader_t**    reader,
                          ockam_writer_t**    writer,
                          ockam_ip_address_t* remote_address);
  ockam_error_t (*deinit)(ockam_transport_t* transport);
} ockam_transport_vtable_t;

struct ockam_transport_t {
  ockam_transport_vtable_t* vtable;
  void*                     ctx;
};

#endif
