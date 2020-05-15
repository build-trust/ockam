#ifndef ockam_io_h
#define ockam_io_h
#include <stdlib.h>
#include <stdint.h>
#include "ockam/error.h"

#define IO_ERROR_INVALID_READER (OCKAM_ERROR_INTERFACE_IO | 1u)

typedef struct ockam_reader_t ockam_reader_t;

typedef struct ockam_writer_t ockam_writer_t;

ockam_error_t ockam_read(ockam_reader_t* reader, uint8_t* buffer, size_t buffer_size, size_t* buffer_length);
ockam_error_t ockam_write(ockam_writer_t* writer, uint8_t* buffer, size_t buffer_length);

#endif