#ifndef io_if_h
#define io_if_h

#include "ockam/io.h"

struct ockam_reader_t {
  ockam_error_t (*read)(void*, uint8_t*, size_t, size_t*);
  void* ctx;
};

struct ockam_writer_t {
  ockam_error_t (*write)(void*, uint8_t*, size_t);
  void* ctx;
};

#endif
