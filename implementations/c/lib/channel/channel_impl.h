#ifndef CHANNEL_IMPL_H
#define CHANNEL_IMPL_H

#include <stdio.h>
#include "ockam/memory.h"
#include "memory/stdlib/stdlib.h"
#include "ockam/transport.h"
#include "ockam/key_agreement.h"

#define MAX_CHANNEL_PACKET_SIZE 0x7fffu

typedef enum {
  CHANNEL_STATE_M1     = 1,
  CHANNEL_STATE_M2     = 2,
  CHANNEL_STATE_M3     = 3,
  CHANNEL_STATE_SECURE = 4
} channel_state_t;

struct ockam_channel_t {
  channel_state_t      state;
  ockam_reader_t*      transport_reader;
  ockam_writer_t*      transport_writer;
  ockam_reader_t*      channel_reader;
  ockam_writer_t*      channel_writer;
  ockam_memory_t*      memory;
  ockam_vault_t*       vault;
  key_establishment_xx key;
};

#endif