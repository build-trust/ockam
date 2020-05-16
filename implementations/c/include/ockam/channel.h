#ifndef OCKAM_CHANNEL_H
#define OCKAM_CHANNEL_H
#include "ockam/error.h"
#include "ockam/io.h"
#include "ockam/transport.h"
#include "ockam/memory.h"
#include "ockam/vault.h"

#define CHANNEL_ERROR_PARAMS (OCKAM_ERROR_INTERFACE_CHANNEL | 0x0001u)

typedef struct ockam_channel_t ockam_channel_t;

typedef struct ockam_channel_attributes_t {
  ockam_reader_t* reader;
  ockam_writer_t* writer;
  ockam_memory_t* memory;
  ockam_vault_t*  vault;
} ockam_channel_attributes_t;

ockam_error_t ockam_channel_init(ockam_channel_t** channel, ockam_channel_attributes_t* p_attrs);
ockam_error_t ockam_channel_connect(ockam_channel_t* p_channel, ockam_reader_t** p_reader, ockam_writer_t** p_writer);
ockam_error_t ockam_channel_accept(ockam_channel_t* p_channel, ockam_reader_t** p_reader, ockam_writer_t** p_writer);
ockam_error_t ockam_channel_deinit(ockam_channel_t* channel);

#endif
