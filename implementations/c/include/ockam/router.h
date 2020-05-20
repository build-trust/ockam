#ifndef OCKAM_ROUTER_H
#define OCKAM_ROUTERL_H
#include "ockam/error.h"
#include "ockam/io.h"
#include "ockam/transport.h"
#include "ockam/memory.h"
#include "ockam/vault.h"
#include "ockam/channel.h"

#define MAX_ROUTER_INPUT 2048

#define ROUTER_ERROR_PARAMS (OCKAM_ERROR_INTERFACE_ROUTER | 0x0001u)

typedef struct ockam_router_t ockam_router_t;

typedef struct ockam_channel_attributes_t {
  ockam_reader_t*    reader;
  ockam_writer_t*    writer;
  ockam_memory_t*    memory;
  ockam_vault_t*     vault;
  ockam_ip_address_t address_in;
  ockam_ip_address_t address_out;
} ockam_router_attributes_t;

ockam_error_t ockam_router_init(ockam_router_t** router, ockam_router_attributes_t* p_attrs);
ockam_error_t ockam_router_connect(ockam_router_t* p_router, ockam_reader_t** p_reader, ockam_writer_t** p_writer);
ockam_error_t ockam_router_accept(ockam_router_t* p_router, ockam_reader_t** p_reader, ockam_writer_t** p_writer);
ockam_error_t ockam_router_deinit(ockam_router_t* router);

#endif
