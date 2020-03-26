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
  kErrorNone = 0,
  kCreateSocket = 0x0100,     /*!< Failed to create socket */
  kConnect = 0x0101,          /*!< Failed to connect, check server address   */
  kSend = 0x0102,             /*!< Failed to send data */
  kServerInit = 0x0103,       /*!< Server initialization failed */
  kReceive = 0x0104,          /*!< Receive buffer failed */
  kBadAddress = 0x0105,       /*!< Bad IP address */
  kAcceptConnection = 0x0106, /*!< Socket accept failed  */
  kNotConnected = 0x0107,     /*!< Connection is not connected */
  kBufferTooSmall = 0x0108,   /*!< Supplied buffer too small */
  kTestFailure = 0x0109,      /*!< Error in test program */
  kMalloc = 0x010a,           /*!< Malloc failed */
  kBadParameter = 0x010b      /*!< Bad parameter */
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
