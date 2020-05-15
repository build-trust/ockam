#ifndef OCKAM_TRANSPORT_H
#define OCKAM_TRANSPORT_H

#include <stdint.h>
#include "ockam/error.h"
#include "ockam/io.h"

#define TRANSPORT_ERROR_NONE             0u
#define TRANSPORT_ERROR_SOCKET_CREATE    (OCKAM_ERROR_INTERFACE_TRANSPORT | 0x0000u) /*!< Failed to create socket */
#define TRANSPORT_ERROR_CONNECT          (OCKAM_ERROR_INTERFACE_TRANSPORT | 0x0001u) /*!< Failed to connect  */
#define TRANSPORT_ERROR_SEND             (OCKAM_ERROR_INTERFACE_TRANSPORT | 0x0002u) /*!< Failed to send data */
#define TRANSPORT_ERROR_SERVER_INIT      (OCKAM_ERROR_INTERFACE_TRANSPORT | 0x0003u) /*!< Server initialization failed */
#define TRANSPORT_ERROR_RECEIVE          (OCKAM_ERROR_INTERFACE_TRANSPORT | 0x0004u) /*!< Receive buffer failed */
#define TRANSPORT_ERROR_BAD_ADDRESS      (OCKAM_ERROR_INTERFACE_TRANSPORT | 0x0005u) /*!< Bad IP address */
#define TRANSPORT_ERROR_ACCEPT           (OCKAM_ERROR_INTERFACE_TRANSPORT | 0x0006u) /*!< Socket accept failed  */
#define TRANSPORT_ERROR_NOT_CONNECTED    (OCKAM_ERROR_INTERFACE_TRANSPORT | 0x0007u) /*!< Connection is not connected */
#define TRANSPORT_ERROR_BUFFER_TOO_SMALL (OCKAM_ERROR_INTERFACE_TRANSPORT | 0x0008u) /*!< Supplied buffer too small */
#define TRANSPORT_ERROR_TEST             (OCKAM_ERROR_INTERFACE_TRANSPORT | 0x0009u) /*!< Error in test program */
#define TRANSPORT_ERROR_BAD_PARAMETER    (OCKAM_ERROR_INTERFACE_TRANSPORT | 0x000Au) /*!< Bad parameter */
#define TRANSPORT_ERROR_ALLOC            (OCKAM_ERROR_INTERFACE_TRANSPORT | 0x000Bu)
#define TRANSPORT_ERROR_MORE_DATA        (OCKAM_ERROR_INTERFACE_TRANSPORT | 0x000Cu) /*!< More data available on socket */
#define TRANSPORT_ERROR_LISTEN           (OCKAM_ERROR_INTERFACE_TRANSPORT | 0x000Du)
#define TRANSPORT_ERROR_SOCKET           (OCKAM_ERROR_INTERFACE_TRANSPORT | 0x000Eu)
/*
 * Transport
 */

typedef struct ockam_transport_t ockam_transport_t;

#define MAX_DNS_NAME_LENGTH     254 // Maximum DNS name length, including terminating NULL
#define MAX_IP_ADDRESS_LENGTH   48  // Maximum length of text DNS address in "xxx.xxx.xxx" format
#define DEFAULT_TCP_LISTEN_PORT 8000

/**
 * OckamInternetAddress - User-friendly internet addresses, includes
 * terminating NULL
 */
typedef struct ockam_ip_address_t {
  uint8_t  dns_name[MAX_DNS_NAME_LENGTH];     // "www.name.ext"
  uint8_t  ip_address[MAX_IP_ADDRESS_LENGTH]; //"xxx.xxx.xxx.xxx"
  uint16_t port;
} ockam_ip_address_t;

ockam_error_t ockam_transport_connect(ockam_transport_t*  transport,
                                      ockam_reader_t**    reader,
                                      ockam_writer_t**    writer,
                                      ockam_ip_address_t* remote_address);
ockam_error_t ockam_transport_accept(ockam_transport_t*  transport,
                                     ockam_reader_t**    reader,
                                     ockam_writer_t**    writer,
                                     ockam_ip_address_t* remote_address);
ockam_error_t ockam_transport_deinit(ockam_transport_t* transport);
/*
 * tcp socket specific transport
 */
typedef struct ockam_transport_tcp_socket_attributes_t {
  ockam_ip_address_t listen_address;
} ockam_transport_tcp_socket_attributes_t;

ockam_error_t ockam_transport_socket_tcp_init(ockam_transport_t**                      transport,
                                              ockam_transport_tcp_socket_attributes_t* attrs);

#endif