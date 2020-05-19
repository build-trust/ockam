#include <stdint.h>
#include <string.h>
#include "ockam/syslog.h"
#include "ockam/error.h"
#include "ockam/codec.h"

//!!todo test this code!

uint8_t* encode_route(uint8_t* p_encoded, codec_route_t* p_routes )
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if(!p_encoded) { error = CODEC_ERROR_PARAMETER; goto exit; }

  *p_encoded++ = p_routes->count_addresses;

  for(int i = 0; i < p_routes->count_addresses; ++i, ++p_routes->p_addresses) {
    *p_encoded++ = p_routes->p_addresses[i].type;
    switch (p_routes->p_addresses[i].type) {
      case ADDRESS_TCP:
      case ADDRESS_UDP:
        switch(p_routes->p_addresses[i].socket_address.tcp_address.host_address.type) {
        case HOST_ADDRESS_IPV4:
          *p_encoded++ = p_routes->p_addresses[i].socket_address.tcp_address.host_address.type;
          memcpy(p_encoded,
                 p_routes->p_addresses[i].socket_address.tcp_address.host_address.ip_address.ipv4, IPV4_ADDRESS_SIZE);
          p_encoded += IPV4_ADDRESS_SIZE;
          *(uint16_t*)p_encoded = p_routes->p_addresses[i].socket_address.tcp_address.port;
          p_encoded += sizeof(uint16_t);
          break;
        case HOST_ADDRESS_IPV6:
          *p_encoded++ = p_routes->p_addresses[i].socket_address.tcp_address.host_address.type;
          memcpy(p_encoded,
                 p_routes->p_addresses[i].socket_address.tcp_address.host_address.ip_address.ipv4, IPV6_ADDRESS_SIZE);
          p_encoded += IPV6_ADDRESS_SIZE;
          *(uint16_t*)p_encoded = p_routes->p_addresses[i].socket_address.tcp_address.port;
          p_encoded += sizeof(uint16_t);
          break;
        default:
          error = CODEC_ERROR_NOT_IMPLEMENTED;
          goto exit;
        }
        break;
      default:
        error = CODEC_ERROR_NOT_IMPLEMENTED;
        goto exit;
    }
  }

exit:
  if(error) { log_error(error, __func__ ); p_encoded = NULL; }
  return p_encoded;
}

uint8_t* decode_route(uint8_t* p_encoded, codec_route_t* p_routes, size_t p_routes_size)
{
  ockam_error_t  error = OCKAM_ERROR_NONE;

  p_routes->count_addresses = *p_encoded++;

  for (int i = 0; i < p_routes->count_addresses; ++i) {
    switch (p_routes->p_addresses[i].type) {
    case ADDRESS_TCP:
    case ADDRESS_UDP:
      switch(p_routes->p_addresses[i].socket_address.tcp_address.host_address.type) {
      case HOST_ADDRESS_IPV4:
        p_routes->p_addresses[i].socket_address.tcp_address.host_address.type = *p_encoded++;
        memcpy(p_routes->p_addresses[i].socket_address.tcp_address.host_address.ip_address.ipv4,
               p_encoded, IPV4_ADDRESS_SIZE);
        p_encoded += IPV4_ADDRESS_SIZE;
        p_routes->p_addresses[i].socket_address.tcp_address.port = *(uint16_t*)p_encoded;
        p_encoded += sizeof(uint16_t);
        break;
      case HOST_ADDRESS_IPV6:
        p_routes->p_addresses[i].socket_address.tcp_address.host_address.type = *p_encoded++;
        memcpy(p_routes->p_addresses[i].socket_address.tcp_address.host_address.ip_address.ipv6,
               p_encoded, IPV6_ADDRESS_SIZE);
        p_encoded += IPV6_ADDRESS_SIZE;
        p_routes->p_addresses[i].socket_address.tcp_address.port = *(uint16_t*)p_encoded;
        p_encoded += sizeof(uint16_t);
        break;
      default:
        error = CODEC_ERROR_NOT_IMPLEMENTED;
        goto exit;
      }
      break;
    default:
      error = CODEC_ERROR_NOT_IMPLEMENTED;
      goto exit;
    }
  }

exit:
  if(error) { log_error(error, __func__ ); p_encoded = NULL; }
  return p_encoded;
}