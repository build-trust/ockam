#include <stdint.h>
#include "ockam/error.h"

#define CODEC_MAX_VLU2_SIZE 0x3fffu

#define AEAD_AES_GCM_TAG_SIZE 16
#define KEY_CURVE_SIZE        32
#define IPV6_ADDRESS_SIZE     16
#define IPV4_ADDRESS_SIZE     4

typedef enum {
  kPing              = 0,
  kPong              = 1,
  kPayload           = 2,
  kPayloadAeadAesGcm = 3,
  kKeyAgreementM1    = 4,
  kKeyAgreementM2    = 5,
  kKeyAgreementM3    = 6
} CodecBodyType;

typedef struct {
  uint16_t encrypted_data_size;
  uint16_t encrypted_data_length;
  uint8_t  tag[AEAD_AES_GCM_TAG_SIZE];
  uint8_t* encrypted_data;
} KTAeadAesGcmPayload;

typedef struct {
  uint16_t data_length;
  uint8_t* data;
} KTPayload;

typedef enum {
  kInvalidParams      = kOckamErrorCodec | 0x0001u,
  kBufferInsufficient = kOckamErrorCodec | 0x0002u
} OckamCodecError;

typedef enum {
  kCurve25519            = 1,
  kCurveP256CompressedY0 = 2,
  kCurveP256CompressedY1 = 3,
  kCurveP256Uncompressed = 4
} CodecKeyCurveType;

typedef struct {
  CodecKeyCurveType type;
  uint8_t           x[KEY_CURVE_SIZE];
  uint8_t           y[KEY_CURVE_SIZE];
} KTPublicKey;

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
  KTPublicKey public_key;
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

uint8_t* decode_variable_length_encoded_u2le(uint8_t* in, uint16_t* val);
uint8_t* encode_variable_length_encoded_u2le(uint8_t* out, uint16_t val);
uint8_t* encode_payload_aead_aes_gcm(uint8_t* encoded, KTAeadAesGcmPayload* payload);
uint8_t* decode_payload_aead_aes_gcm(uint8_t* encoded, KTAeadAesGcmPayload* payload);
uint8_t* encode_public_key(uint8_t* encoded, KTPublicKey* public_key);
uint8_t* decode_public_key(uint8_t* encoded, KTPublicKey* public_key);
uint8_t* encode_payload(uint8_t* encoded, KTPayload* kt_payload);
uint8_t* decode_payload(uint8_t* encoded, KTPayload* kt_payload);
uint8_t* encode_endpoint(uint8_t* encoded, CodecEndpointType type, uint8_t* endpoint);
uint8_t* decode_endpoint(uint8_t* encoded, CodecEndpointType* type, uint8_t* endpoint);
