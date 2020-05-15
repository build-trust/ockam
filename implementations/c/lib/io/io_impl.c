#ifndef OCKAM_IO_H
#define OCKAM_IO_H
#include "ockam/syslog.h"
#include "ockam/io/io_impl.h"

ockam_error_t ockam_read(ockam_reader_t* p_reader, uint8_t* buffer, size_t buffer_size, size_t* buffer_length)
{
  ockam_error_t error;

  if (!p_reader) {
    error = IO_ERROR_INVALID_READER;
    goto exit;
  }
  error = p_reader->read(p_reader->ctx, buffer, buffer_size, buffer_length);

exit:
  if (error) log_error(error, "ockam_read");
  return error;
}
ockam_error_t ockam_write(ockam_writer_t* p_writer, uint8_t* buffer, size_t buffer_length)
{
  ockam_error_t error;

  if (!p_writer) {
    error = IO_ERROR_INVALID_READER;
    goto exit;
  }
  error = p_writer->write(p_writer->ctx, buffer, buffer_length);

exit:
  if (error) log_error(error, "ockam_write");
  return error;
}

#endif
