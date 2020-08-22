#ifndef OCKAM_TRANSPORT_IMPL_H
#include "ockam/codec.h"

#define OCKAM_TRANSPORT_IMPL_H
#define MAX_DNS_NAME_LENGTH   254 // Maximum DNS name length, including terminating NULL
#define MAX_IP_ADDRESS_LENGTH 48  // Maximum length of text DNS address in "xxx.xxx.xxx" format

/**
 * OckamInternetAddress - User-friendly internet addresses, includes
 * terminating NULL
 */
typedef struct ockam_ip_address_t {
  uint8_t  dns_name[MAX_DNS_NAME_LENGTH];     // "www.name.ext"
  uint8_t  ip_address[MAX_IP_ADDRESS_LENGTH]; //"xxx.xxx.xxx.xxx"
  uint16_t port;
} ockam_ip_address_t;

struct ockam_transport {
  struct ockam_transport_vtable* vtable;
  void*                          ctx;
};

typedef struct ockam_transport_vtable {
  ockam_error_t (*connect)(
    void* ctx, ockam_reader_t** reader, ockam_writer_t** writer, int16_t retry_count, uint16_t retry_interval);
  ockam_error_t (*accept)(void*               ctx,
                          ockam_reader_t**    reader,
                          ockam_writer_t**    writer,
                          ockam_ip_address_t* remote_address);
  ockam_error_t (*get_local_address)(void* ctx, codec_address_t*);
  ockam_error_t (*get_remote_address)(void* ctx, codec_address_t*);
  ockam_error_t (*deinit)(struct ockam_transport* transport);
} ockam_transport_vtable_t;

#endif
