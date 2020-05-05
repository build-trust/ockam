/**
 ********************************************************************************************************
 * @file        transport.h
 * @brief       Public-facing API function prototypes for Ockam's transport
 *library
 ********************************************************************************************************
 */
#ifndef OCKAM_TRANSPORT_H
#define OCKAM_TRANSPORT_H 1

/*
 ********************************************************************************************************
 *                                             INCLUDE FILES *
 ********************************************************************************************************
 */
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#include "ockam/error.h"

/*
 ********************************************************************************************************
 *                                                DEFINES *
 ********************************************************************************************************
 */

#define MAX_DNS_NAME_LENGTH 254    // Maximum DNS name length, including terminating NULL
#define MAX_DNS_ADDRESS_LENGTH 48  // Maximum length of text DNS address in "xxx.xxx.xxx" format
#define DEFAULT_TCP_LISTEN_PORT 8000

typedef enum {
  kErrorNone = 0u,
  kCreateSocket = kOckamErrorInterfaceTransport | 0x0100u,     /*!< Failed to create socket */
  kConnect = kOckamErrorInterfaceTransport | 0x0101u,          /*!< Failed to connect, check server address   */
  kSend = kOckamErrorInterfaceTransport | 0x0102u,             /*!< Failed to send data */
  kServerInit = kOckamErrorInterfaceTransport | 0x0103u,       /*!< Server initialization failed */
  kReceive = kOckamErrorInterfaceTransport | 0x0104u,          /*!< Receive buffer failed */
  kBadAddress = kOckamErrorInterfaceTransport | 0x0105u,       /*!< Bad IP address */
  kAcceptConnection = kOckamErrorInterfaceTransport | 0x0106u, /*!< Socket accept failed  */
  kNotConnected = kOckamErrorInterfaceTransport | 0x0107u,     /*!< Connection is not connected */
  kBufferTooSmall = kOckamErrorInterfaceTransport | 0x0108u,   /*!< Supplied buffer too small */
  kTestFailure = kOckamErrorInterfaceTransport | 0x0109u,      /*!< Error in test program */
  kMalloc = kOckamErrorInterfaceTransport | 0x010Au,           /*!< Malloc failed */
  kBadParameter = kOckamErrorInterfaceTransport | 0x010Bu,     /*!< Bad parameter */
  kMoreData = kOckamErrorInterfaceTransport | 0x010Cu          /*!< More data available on socket */
} TransportError;

/*
 ********************************************************************************************************
 *                                        PUBLIC DATA TYPES *
 ********************************************************************************************************
 */

/**
 * OckamInternetAddress - User-friendly internet addresses, includes
 * terminating NULL
 */
typedef struct {
  char dnsName[MAX_DNS_NAME_LENGTH];       // "www.name.ext"
  char IPAddress[MAX_DNS_ADDRESS_LENGTH];  //"xxx.xxx.xxx.xxx"
  uint16_t port;
} OckamInternetAddress;

/**
 * OckamTransport represents a communication channel between two
 * entities. Virtually every call into the transport library takes an
 * OckamTransport as an argument.
 */
typedef void *OckamTransportCtx;

/**
 * OckamTransportTCPConfig holds configuration options specific to TCP
 * transport types
 */

typedef struct {
  enum { kBlocking, kNonBlocking } block;
} OckamTransportConfig;

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
  // Initialize
  TransportError (*Create)(OckamTransportCtx *, OckamTransportConfig *);

  // listen functions
  TransportError (*Listen)(OckamTransportCtx, OckamInternetAddress *, OckamTransportCtx *);

  // connect functions
  TransportError (*Connect)(OckamTransportCtx connection, OckamInternetAddress *address);

  // receive functions
  TransportError (*Read)(OckamTransportCtx connection, void *buffer, uint16_t length, uint16_t *p_bytesReceived);

  // send functions
  TransportError (*Write)(OckamTransportCtx connection, void *buffer, uint16_t length);

  // uninit
  TransportError (*Destroy)(OckamTransportCtx connection);
} OckamTransport;

#ifdef __cplusplus
}
#endif

#endif
