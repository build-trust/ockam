#ifndef OCKAM_CHANNEL_H
#define OCKAM_CHANNEL_H
#include "ockam/error.h"
#include "ockam/io.h"
#include "ockam/transport.h"
#include "ockam/memory.h"
#include "ockam/vault.h"
#include "ockam/channel/channel_impl.h"
#include "ockam/codec.h"

extern const char* const OCKAM_CHANNEL_INTERFACE_ERROR_DOMAIN;

typedef enum {
  OCKAM_CHANNEL_INTERFACE_ERROR_INVALID_PARAM   = 1,
  OCKAM_CHANNEL_INTERFACE_ERROR_NOT_IMPLEMENTED = 2,
  OCKAM_CHANNEL_INTERFACE_ERROR_KEY_AGREEMENT   = 3,
  OCKAM_CHANNEL_INTERFACE_ERROR_STATE           = 4,
  OCKAM_CHANNEL_INTERFACE_ERROR_READ_PENDING    = 5,
  OCKAM_CHANNEL_INTERFACE_ERROR_WRITE_PENDING   = 6,
} ockam_error_code_channel_interface_t;

extern const ockam_error_t ockam_channel_interface_error_none;

typedef struct ockam_channel_t ockam_channel_t;

typedef struct ockam_channel_attributes_t {
  ockam_reader_t* reader;
  ockam_writer_t* writer;
  ockam_memory_t* memory;
  ockam_vault_t*  vault;
  codec_route_t   route;
  codec_address_t route_addresses[MAX_HOPS];
  codec_address_t local_host_address;
} ockam_channel_attributes_t;

/*
 * For now we only support one pending read and one pending write per channel.
 */

typedef struct ockam_channel_poll_result {
  uint8_t  channel_is_secure;
  uint8_t* write_buffer;
  size_t   bytes_written;
  uint8_t* read_buffer;
  size_t   bytes_read;
} ockam_channel_poll_result_t;

ockam_error_t ockam_channel_init(ockam_channel_t* channel, ockam_channel_attributes_t* p_attrs);
ockam_error_t
              ockam_channel_connect(ockam_channel_t* ch, codec_route_t* route, ockam_reader_t** p_reader, ockam_writer_t** p_writer);
ockam_error_t ockam_channel_accept(ockam_channel_t* p_channel, ockam_reader_t** p_reader, ockam_writer_t** p_writer);
ockam_error_t ockam_channel_poll(ockam_channel_t* p_channel, ockam_channel_poll_result_t* result);
ockam_error_t ockam_channel_deinit(ockam_channel_t* channel);

#endif
