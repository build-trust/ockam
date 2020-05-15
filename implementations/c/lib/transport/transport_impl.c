#include "ockam/io.h"
#include "ockam/transport.h"
#include "transport_impl.h"

ockam_error_t ockam_transport_connect(ockam_transport_t*  transport,
                                      ockam_reader_t**    reader,
                                      ockam_writer_t**    writer,
                                      ockam_ip_address_t* remote_address)
{
  return transport->vtable->connect(transport->ctx, reader, writer, remote_address);
}
ockam_error_t ockam_transport_accept(ockam_transport_t*  transport,
                                     ockam_reader_t**    reader,
                                     ockam_writer_t**    writer,
                                     ockam_ip_address_t* remote_address)
{
  return transport->vtable->accept(transport->ctx, reader, writer, remote_address);
}

ockam_error_t ockam_transport_deinit(ockam_transport_t* transport)
{
  return transport->vtable->deinit(transport);
}