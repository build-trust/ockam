#include <stdint.h>
#include <string.h>
#include "ockam/error.h"
#include "ockam/codec.h"

uint8_t* encode_endpoint(uint8_t* encoded, CodecEndpointType type, uint8_t* endpoint)
{
  *encoded++ = (uint8_t) type;

  switch (type) {
  case kLocal: {
    KTLocalEndpoint* local_endpoint = (KTLocalEndpoint*) endpoint;
    encoded                         = encode_variable_length_encoded_u2le(encoded, local_endpoint->length);
    if (0 == encoded) goto exit_block;
    memcpy(encoded, local_endpoint->data, local_endpoint->length);
    encoded += local_endpoint->length;
    break;
  }
  case kChannel: {
    KTChannelEndpoint* channel_endpoint = (KTChannelEndpoint*) endpoint;
    encoded                             = encode_public_key(encoded, &channel_endpoint->public_key);
    if (0 == encoded) goto exit_block;
    break;
  }
  case kTcpIpv4: {
    KTTcpIpv4Endpoint* tcp_ipv_4_endpoint = (KTTcpIpv4Endpoint*) endpoint;
    memcpy(encoded, tcp_ipv_4_endpoint, sizeof(KTTcpIpv4Endpoint));
    encoded += sizeof(KTTcpIpv4Endpoint);
    break;
  }
  case kUdpIpv4: {
    KTUdpIpv4Endpoint* ucp_ipv_4_endpoint = (KTUdpIpv4Endpoint*) endpoint;
    memcpy(encoded, ucp_ipv_4_endpoint, sizeof(KTUdpIpv4Endpoint));
    encoded += sizeof(kUdpIpv4);
    break;
  }
  case kTcpIpv6: {
    KTTcpIpv6Endpoint* tcp_ipv_6_endpoint = (KTTcpIpv6Endpoint*) endpoint;
    memcpy(encoded, tcp_ipv_6_endpoint, sizeof(KTTcpIpv6Endpoint));
    encoded += sizeof(kTcpIpv6);
    break;
  }
  case kUdpIpv6: {
    KTUdpIpv6Endpoint* ucp_ipv_6_endpoint = (KTUdpIpv6Endpoint*) endpoint;
    memcpy(encoded, ucp_ipv_6_endpoint, sizeof(KTUdpIpv6Endpoint));
    encoded += sizeof(kUdpIpv6);
    break;
  }
  case kInvalid:
  default: {
    encoded = 0;
  }
  };

exit_block:
  return encoded;
}

uint8_t* decode_endpoint(uint8_t* encoded, CodecEndpointType* type, uint8_t* endpoint)
{
  *type = *encoded++;

  switch (*type) {
  case kLocal: {
    KTLocalEndpoint* local_endpoint = (KTLocalEndpoint*) endpoint;
    encoded                         = decode_variable_length_encoded_u2le(encoded, &local_endpoint->length);
    if (0 == encoded) goto exit_block;
    memcpy(local_endpoint->data, encoded, local_endpoint->length);
    encoded += local_endpoint->length;
    break;
  }
  case kChannel: {
    KTChannelEndpoint* channel_endpoint = (KTChannelEndpoint*) endpoint;
    encoded                             = decode_public_key(encoded, &channel_endpoint->public_key);
    if (0 == encoded) goto exit_block;
    break;
  }
  case kTcpIpv4: {
    KTTcpIpv4Endpoint* tcp_ipv_4_endpoint = (KTTcpIpv4Endpoint*) endpoint;
    memcpy(tcp_ipv_4_endpoint, encoded, sizeof(KTTcpIpv4Endpoint));
    encoded += sizeof(KTTcpIpv4Endpoint);
    break;
  }
  case kUdpIpv4: {
    KTUdpIpv4Endpoint* ucp_ipv_4_endpoint = (KTUdpIpv4Endpoint*) endpoint;
    memcpy(encoded, ucp_ipv_4_endpoint, sizeof(KTUdpIpv4Endpoint));
    encoded += sizeof(kUdpIpv4);
    break;
  }
  case kTcpIpv6: {
    KTTcpIpv6Endpoint* tcp_ipv_6_endpoint = (KTTcpIpv6Endpoint*) endpoint;
    memcpy(tcp_ipv_6_endpoint, encoded, sizeof(KTTcpIpv6Endpoint));
    encoded += sizeof(kTcpIpv6);
    break;
  }
  case kUdpIpv6: {
    KTUdpIpv6Endpoint* ucp_ipv_6_endpoint = (KTUdpIpv6Endpoint*) endpoint;
    memcpy(ucp_ipv_6_endpoint, encoded, sizeof(KTUdpIpv6Endpoint));
    encoded += sizeof(kUdpIpv6);
    break;
  }
  case kInvalid:
  default:
    encoded = 0;
  };

exit_block:
  return encoded;
}
