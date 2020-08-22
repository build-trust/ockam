#ifndef CHANNEL_IMPL_H
#define CHANNEL_IMPL_H

#include "ockam/io.h"
#include "ockam/vault.h"
#include "ockam/memory.h"
#include "ockam/codec.h"
#include "ockam/key_agreement/impl.h"

#define MAX_CHANNEL_PACKET_SIZE 0x7fffu
#define MAX_HOPS                5

extern void string_to_hex(uint8_t* hexstring, uint8_t* val, size_t* p_bytes);
extern void print_uint8_str(uint8_t* p, uint16_t size, char* msg);

typedef enum {
  CHANNEL_STATE_UNSECURE = 0,
  CHANNEL_STATE_M1       = 1,
  CHANNEL_STATE_M2       = 2,
  CHANNEL_STATE_M3       = 3,
  CHANNEL_STATE_SECURE   = 4
} channel_state_t;

struct ockam_channel_t {
  channel_state_t state;
  ockam_reader_t* transport_reader;
  ockam_writer_t* transport_writer;
  ockam_reader_t* channel_reader;
  ockam_writer_t* channel_writer;
  ockam_vault_t*  vault;
  codec_address_t local_host_address;
  codec_address_t local_address;
  codec_route_t   onward_route;
  codec_address_t onward_addresses[MAX_HOPS];
  uint8_t*        channel_read_buffer;
  size_t          channel_read_buffer_length;
  uint8_t*        channel_write_buffer;
  uint8_t*        app_read_buffer;
  size_t          app_read_buffer_size;
  size_t          app_read_buffer_length;
  uint8_t*        app_write_buffer;
  size_t          app_write_buffer_length;
  ockam_key_t     key;
};

#endif
