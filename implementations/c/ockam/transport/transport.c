#include "ockam/io.h"
#include "ockam/transport.h"
#include "ockam/transport/impl.h"
#include "ockam/memory.h"

ockam_memory_t* gp_ockam_transport_memory = NULL;

ockam_error_t ockam_transport_connect(ockam_transport_t* transport,
                                      ockam_reader_t**   reader,
                                      ockam_writer_t**   writer,
                                      int16_t            retry_count,
                                      uint16_t           retry_interval)
{
  return transport->vtable->connect(transport->ctx, reader, writer, retry_count, retry_interval);
}
ockam_error_t ockam_transport_accept(ockam_transport_t*  transport,
                                     ockam_reader_t**    reader,
                                     ockam_writer_t**    writer,
                                     ockam_ip_address_t* remote_address)
{
  return transport->vtable->accept(transport->ctx, reader, writer, remote_address);
}

ockam_error_t ockam_get_local_address(ockam_transport_t* transport, codec_address_t* address)
{
  return transport->vtable->get_local_address(transport->ctx, address);
}

ockam_error_t ockam_get_remote_address(ockam_transport_t* transport, codec_address_t* address)
{
  return transport->vtable->get_remote_address(transport->ctx, address);
}

ockam_error_t ockam_transport_deinit(ockam_transport_t* transport)
{
  return transport->vtable->deinit(transport);
}
