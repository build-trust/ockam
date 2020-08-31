#include <stdint.h>
#include <string.h>
#include "ockam/log.h"
#include "ockam/error.h"
#include "ockam/codec.h"

uint8_t* encode_route(uint8_t* p_encoded, codec_route_t* p_route)
{
  ockam_error_t error = ockam_codec_error_none;

  if (!p_encoded) {
    error.code = OCKAM_CODEC_ERROR_INVALID_PARAM;
    goto exit;
  }

  *p_encoded++ = p_route->count_addresses;

  for (int i = 0; i < p_route->count_addresses; ++i) {
    *p_encoded++ = p_route->p_addresses[i].type;
    switch (p_route->p_addresses[i].type) {
    case ADDRESS_TCP:
    case ADDRESS_UDP:
      switch (p_route->p_addresses[i].address.socket_address.tcp_address.host_address.type) {
      case HOST_ADDRESS_IPV4:
        *p_encoded++ = p_route->p_addresses[i].address.socket_address.tcp_address.host_address.type;
        memcpy(p_encoded,
               p_route->p_addresses[i].address.socket_address.tcp_address.host_address.ip_address.ipv4,
               IPV4_ADDRESS_SIZE);
        p_encoded += IPV4_ADDRESS_SIZE;
        *(uint16_t*) p_encoded = p_route->p_addresses[i].address.socket_address.tcp_address.port;
        p_encoded += sizeof(uint16_t);
        break;
      case HOST_ADDRESS_IPV6:
        *p_encoded++ = p_route->p_addresses[i].address.socket_address.tcp_address.host_address.type;
        memcpy(p_encoded,
               p_route->p_addresses[i].address.socket_address.tcp_address.host_address.ip_address.ipv4,
               IPV6_ADDRESS_SIZE);
        p_encoded += IPV6_ADDRESS_SIZE;
        *(uint16_t*) p_encoded = p_route->p_addresses[i].address.socket_address.tcp_address.port;
        p_encoded += sizeof(uint16_t);
        break;
      default:
        error.code = OCKAM_CODEC_ERROR_NOT_IMPLEMENTED;
        goto exit;
      }
      break;
    case ADDRESS_LOCAL:
      *p_encoded++ = p_route->p_addresses[i].address.local_address.size;
      for (uint8_t j = 0; j < p_route->p_addresses[i].address.local_address.size; ++j) {
        *p_encoded++ = p_route->p_addresses[i].address.local_address.address[j];
      }
      break;
    default:
      error.code = OCKAM_CODEC_ERROR_NOT_IMPLEMENTED;
      goto exit;
    }
  }

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return p_encoded;
}

uint8_t* decode_route(uint8_t* p_encoded, codec_route_t* p_route)
{
  ockam_error_t error = ockam_codec_error_none;

  p_route->count_addresses = *p_encoded++;

  for (int i = 0; i < p_route->count_addresses; ++i) {
    p_route->p_addresses[i].type = *p_encoded++;
    switch (p_route->p_addresses[i].type) {
    case ADDRESS_TCP:
    case ADDRESS_UDP:
      p_route->p_addresses[i].address.socket_address.tcp_address.host_address.type = *p_encoded++;
      switch (p_route->p_addresses[i].address.socket_address.tcp_address.host_address.type) {
      case HOST_ADDRESS_IPV4:
        memcpy(p_route->p_addresses[i].address.socket_address.tcp_address.host_address.ip_address.ipv4,
               p_encoded,
               IPV4_ADDRESS_SIZE);
        p_encoded += IPV4_ADDRESS_SIZE;
        p_route->p_addresses[i].address.socket_address.tcp_address.port = *(uint16_t*) p_encoded;
        p_encoded += sizeof(uint16_t);
        break;
      case HOST_ADDRESS_IPV6:
        memcpy(p_route->p_addresses[i].address.socket_address.tcp_address.host_address.ip_address.ipv6,
               p_encoded,
               IPV6_ADDRESS_SIZE);
        p_encoded += IPV6_ADDRESS_SIZE;
        p_route->p_addresses[i].address.socket_address.tcp_address.port = *(uint16_t*) p_encoded;
        p_encoded += sizeof(uint16_t);
        break;
      default:
        error.code = OCKAM_CODEC_ERROR_NOT_IMPLEMENTED;
        goto exit;
      }
      break;
    case ADDRESS_LOCAL:
      p_route->p_addresses[i].address.local_address.size = *p_encoded++;
      for (uint8_t j = 0; j < p_route->p_addresses[i].address.local_address.size; ++j) {
        p_route->p_addresses[i].address.local_address.address[j] = *p_encoded++;
      }
      break;
    default:
      error.code = OCKAM_CODEC_ERROR_NOT_IMPLEMENTED;
      goto exit;
    }
  }

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return p_encoded;
}
