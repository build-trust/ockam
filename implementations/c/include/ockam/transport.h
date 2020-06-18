#ifndef OCKAM_TRANSPORT_H
#define OCKAM_TRANSPORT_H

#include <stdint.h>
#include "ockam/error.h"
#include "ockam/io.h"
#include "transport/transport_impl.h"
#include "ockam/memory.h"

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
#define TRANSPORT_ERROR_INVALID_OP       (OCKAM_ERROR_INTERFACE_TRANSPORT | 0x000Fu)
/*
 * Transport
 */

typedef struct ockam_transport ockam_transport_t;

ockam_error_t ockam_transport_connect(ockam_transport_t*  transport,
                                      ockam_reader_t**    reader,
                                      ockam_writer_t**    writer,
                                      ockam_ip_address_t* remote_address,
                                      int16_t  retry_count,   // -1 : forever, 0 : no retries, >0 : number of retries
                                      uint16_t retry_interval // in seconds
);
ockam_error_t ockam_transport_accept(ockam_transport_t*  transport,
                                     ockam_reader_t**    reader,
                                     ockam_writer_t**    writer,
                                     ockam_ip_address_t* remote_address);
ockam_error_t ockam_transport_deinit(ockam_transport_t* transport);
/*
 * tcp socket specific transport
 */
typedef struct ockam_transport_socket_attributes {
  ockam_ip_address_t listen_address;
  ockam_memory_t*    p_memory;
} ockam_transport_socket_attributes_t;

ockam_error_t ockam_transport_socket_udp_init(ockam_transport_t*                   p_transport,
                                              ockam_transport_socket_attributes_t* p_cfg);

ockam_error_t ockam_transport_socket_tcp_init(ockam_transport_t* transport, ockam_transport_socket_attributes_t* attrs);

#endif