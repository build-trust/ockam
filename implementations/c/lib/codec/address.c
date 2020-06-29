#include <stdint.h>
#include <string.h>
#include "ockam/syslog.h"
#include "ockam/error.h"
#include "ockam/codec.h"

uint8_t* encode_address(uint8_t* p_encoded, codec_address_t* p_address)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  if (!p_encoded) {
    error = CODEC_ERROR_PARAMETER;
    goto exit;
  }

  *p_encoded++ = p_address->type;
  switch (p_address->type) {
  case ADDRESS_TCP:
  case ADDRESS_UDP:
    switch (p_address->address.tcp_address.host_address.type) {
    case HOST_ADDRESS_IPV4:
      *p_encoded++ = p_address->address.tcp_address.host_address.type;
      memcpy(p_encoded, p_address->address.tcp_address.host_address.ip_address.ipv4, IPV4_ADDRESS_SIZE);
      p_encoded += IPV4_ADDRESS_SIZE;
      *(uint16_t*) p_encoded = p_address->address.tcp_address.port;
      p_encoded += sizeof(uint16_t);
      break;
    case HOST_ADDRESS_IPV6:
      *p_encoded++ = p_address->address.tcp_address.host_address.type;
      memcpy(p_encoded, p_address->address.tcp_address.host_address.ip_address.ipv4, IPV6_ADDRESS_SIZE);
      p_encoded += IPV6_ADDRESS_SIZE;
      *(uint16_t*) p_encoded = p_address->address.tcp_address.port;
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

exit:
  if (error) log_error(error, __func__);
  return p_encoded;
}

uint8_t* decode_address(uint8_t* p_encoded, codec_address_t* p_address)
{
  ockam_error_t error = OCKAM_ERROR_NONE;

  p_address->type = *p_encoded++;

  switch (p_address->type) {
  case ADDRESS_LOCAL:
    memcpy(p_address->address.local_address, p_encoded, LOCAL_ADDRESS_SIZE);
    p_encoded += LOCAL_ADDRESS_SIZE;
    break;
  case ADDRESS_TCP:
  case ADDRESS_UDP:
    p_address->address.tcp_address.host_address.type = *p_encoded++;
    switch (p_address->address.tcp_address.host_address.type) {
    case HOST_ADDRESS_IPV4:
      memcpy(p_address->address.tcp_address.host_address.ip_address.ipv4, p_encoded, IPV4_ADDRESS_SIZE);
      p_encoded += IPV4_ADDRESS_SIZE;
      p_address->address.tcp_address.port = *(uint16_t*) p_encoded;
      p_encoded += sizeof(uint16_t);
      break;
    case HOST_ADDRESS_IPV6:
      memcpy(p_address->address.tcp_address.host_address.ip_address.ipv6, p_encoded, IPV6_ADDRESS_SIZE);
      p_encoded += IPV6_ADDRESS_SIZE;
      p_address->address.tcp_address.port = *(uint16_t*) p_encoded;
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

exit:
  if (error) log_error(error, __func__);
  return p_encoded;
}
