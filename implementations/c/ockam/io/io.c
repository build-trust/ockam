#ifndef OCKAM_IO_H
#define OCKAM_IO_H

#include "ockam/log.h"
#include "ockam/io/impl.h"

const char* const OCKAM_IO_INTERFACE_ERROR_DOMAIN = "OCKAM_IO_INTERFACE_ERROR_DOMAIN";

static const ockam_error_t ockam_io_interface_error_none = {
  OCKAM_ERROR_NONE,
  OCKAM_IO_INTERFACE_ERROR_DOMAIN
};

ockam_error_t ockam_read(ockam_reader_t* p_reader, uint8_t* buffer, size_t buffer_size, size_t* buffer_length)
{
  ockam_error_t error = ockam_io_interface_error_none;

  if (!p_reader) {
    error.code = OCKAM_IO_INTERFACE_ERROR_INVALID_READER;
    goto exit;
  }
  error = p_reader->read(p_reader->ctx, buffer, buffer_size, buffer_length);

exit:
  return error;
}
ockam_error_t ockam_write(ockam_writer_t* p_writer, uint8_t* buffer, size_t buffer_length)
{
  ockam_error_t error = ockam_io_interface_error_none;

  if (!p_writer) {
    error.code = OCKAM_IO_INTERFACE_ERROR_INVALID_WRITER;
    goto exit;
  }
  error = p_writer->write(p_writer->ctx, buffer, buffer_length);

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

#endif
