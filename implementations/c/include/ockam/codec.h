#include <stdint.h>
#include "ockam/error.h"

#define CODEC_ERROR_PARAMETER       (OCKAM_ERROR_INTERFACE_CODEC | 0x0001u)
#define CODEC_ERROR_NOT_IMPLEMENTED (OCKAM_ERROR_INTERFACE_CODEC | 0X0002u)

#define OCKAM_WIRE_PROTOCOL_VERSION 1

#define CODEC_MAX_VLU2_SIZE 0x3fffu

#define AEAD_AES_GCM_TAG_SIZE 16
#define KEY_CURVE_SIZE        32
#define IPV6_ADDRESS_SIZE     16
#define IPV4_ADDRESS_SIZE     4

/*
 *
 *         0: ping
        1: pong

        2: payload

        3: request_channel
        4: key_agreement_t1_m2
        5: key_agreement_t1_m3

 */

typedef enum {
  PING                = 0,
  PONG                = 1,
  PAYLOAD             = 2,
  REQUEST_CHANNEL     = 3,
  KEY_AGREEMENT_T1_M2 = 4,
  KEY_AGREEMENT_T1_M3 = 5,
  kPayloadAeadAesGcm  = 6,
  kKeyAgreementM1     = 7,
  kKeyAgreementM2     = 8,
  kKeyAgreementM3     = 9
} codec_message_type_t;

typedef struct {
  uint16_t encrypted_data_size;
  uint16_t encrypted_data_length;
  uint8_t  tag[AEAD_AES_GCM_TAG_SIZE];
  uint8_t* encrypted_data;
} codec_aead_aes_gcm_payload_t;

typedef struct {
  uint16_t data_length;
  uint8_t* data;
} codec_payload_t;

typedef enum {
  kCurve25519            = 1,
  kCurveP256CompressedY0 = 2,
  kCurveP256CompressedY1 = 3,
  kCurveP256Uncompressed = 4
} codec_key_curve_type_t;

typedef struct {
  codec_key_curve_type_t type;
  uint8_t                x[KEY_CURVE_SIZE];
  uint8_t                y[KEY_CURVE_SIZE];
} codec_public_key_t;

/* Endpoints */

typedef enum {
  kLocal   = 0,
  kChannel = 1,
  kTcpIpv4 = 2,
  kTcpIpv6 = 3,
  kUdpIpv4 = 4,
  kUdpIpv6 = 5,
  kInvalid = 6
} CodecEndpointType;

typedef struct {
  uint16_t length;
  uint8_t* data;
} KTLocalEndpoint;

typedef struct {
  codec_public_key_t public_key;
} KTChannelEndpoint;

typedef struct {
  uint8_t  ip4[IPV4_ADDRESS_SIZE];
  uint16_t port;
} KTTcpIpv4Endpoint;

typedef struct {
  uint8_t  ip6[IPV6_ADDRESS_SIZE];
  uint16_t port;
} KTTcpIpv6Endpoint;

typedef struct {
  uint8_t  ip4[IPV4_ADDRESS_SIZE];
  uint16_t port;
} KTUdpIpv4Endpoint;

typedef struct {
  uint8_t  ip6[IPV6_ADDRESS_SIZE];
  uint16_t port;
} KTUdpIpv6Endpoint;

typedef enum { kSendTo = 0, kReplyTo = 1 } CodecHeaderType;

typedef struct {
  CodecHeaderType   header_type;
  CodecEndpointType endpoint_type;
  uint8_t*          endpoint;
} KTHeader;

typedef enum { ADDRESS_LOCAL = 0, ADDRESS_TCP = 1, ADDRESS_UDP = 2 } codec_address_type_t;

typedef enum { HOST_ADDRESS_IPV4 = 0, HOST_ADDRESS_IPV6 = 1 } codec_host_address_type;

typedef struct {
  codec_host_address_type type;
  union {
    uint8_t ipv4[IPV4_ADDRESS_SIZE];
    uint8_t ipv6[IPV6_ADDRESS_SIZE];
  } ip_address;
} codec_host_address_t;

typedef struct {
  codec_host_address_t host_address;
  uint16_t             port;
} codec_socket_t, codec_tcp_address_t, codec_udp_address_t;

typedef struct {
  codec_address_type_t type;
  union {
    codec_udp_address_t udp_address;
    codec_tcp_address_t tcp_address;
  } socket_address;
} codec_address_t;

typedef struct {
  uint8_t          count_addresses;
  codec_address_t* p_addresses;
} codec_route_t;

uint8_t* decode_variable_length_encoded_u2le(uint8_t* in, uint16_t* val);
uint8_t* encode_variable_length_encoded_u2le(uint8_t* out, uint16_t val);
uint8_t* encode_payload_aead_aes_gcm(uint8_t* encoded, codec_aead_aes_gcm_payload_t* payload);
uint8_t* decode_payload_aead_aes_gcm(uint8_t* encoded, codec_aead_aes_gcm_payload_t* payload);
uint8_t* encode_public_key(uint8_t* encoded, codec_public_key_t* public_key);
uint8_t* decode_public_key(uint8_t* encoded, codec_public_key_t* public_key);
uint8_t* encode_payload(uint8_t* encoded, codec_payload_t* kt_payload);
uint8_t* decode_payload(uint8_t* encoded, codec_payload_t* kt_payload);
uint8_t* encode_endpoint(uint8_t* encoded, CodecEndpointType type, uint8_t* endpoint);
uint8_t* decode_endpoint(uint8_t* encoded, CodecEndpointType* type, uint8_t* endpoint);
uint8_t* encode_key_agreement(uint8_t* encoded, codec_payload_t* kt_payload);
uint8_t* decode_key_agreement(uint8_t* encoded, codec_payload_t* kt_payload);
uint8_t* encode_ockam_wire(uint8_t* p_encoded);
uint8_t* decode_ockam_wire(uint8_t* p_encoded);
uint8_t* encode_route(uint8_t* p_encoded, codec_route_t* p_route);
uint8_t* decode_route(uint8_t* p_encoded, codec_route_t* p_route);
