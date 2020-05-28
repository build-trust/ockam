
#include <stdio.h>
#include <string.h>
#include <stdbool.h>

#include "ockam/error.h"
#include "ockam/memory.h"
#include "ockam/syslog.h"
#include "ockam/transport.h"
#include "ockam/vault.h"
#include "ockam/channel.h"
#include "channel_test.h"

ockam_error_t establish_initiator_transport(ockam_transport_t** transport,
                                            ockam_ip_address_t* ip_address,
                                            ockam_reader_t**    pp_reader,
                                            ockam_writer_t**    pp_writer)
{
  ockam_error_t                           error = TRANSPORT_ERROR_NONE;
  ockam_transport_tcp_socket_attributes_t tcp_attrs;
  memset(&tcp_attrs, 0, sizeof(tcp_attrs));

  error = ockam_transport_socket_tcp_init(transport, &tcp_attrs);
  if (error) goto exit;

  error = ockam_transport_connect(*transport, pp_reader, pp_writer, ip_address);
  if (error) goto exit;

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t channel_initiator(ockam_vault_t* vault, ockam_memory_t* p_memory, ockam_ip_address_t* ip_address)
{
  ockam_error_t              error       = OCKAM_ERROR_NONE;
  ockam_transport_t*         p_transport = NULL;
  ockam_channel_t*           p_channel   = NULL;
  ockam_reader_t*            p_channel_reader;
  ockam_writer_t*            p_channel_writer;
  ockam_reader_t*            p_transport_reader;
  ockam_writer_t*            p_transport_writer;
  uint8_t                    recv_buffer[MAX_XX_TRANSMIT_SIZE];
  size_t                     bytes_received = 0;
  ockam_channel_attributes_t channel_attrs;

  error = establish_initiator_transport(&p_transport, ip_address, &p_transport_reader, &p_transport_writer);
  if (error) goto exit;

  channel_attrs.reader = p_transport_reader;
  channel_attrs.writer = p_transport_writer;
  channel_attrs.memory = p_memory;
  channel_attrs.vault  = vault;

  error = ockam_channel_init(&p_channel, &channel_attrs);
  if (error) goto exit;

  error = ockam_channel_connect(p_channel, &p_channel_reader, &p_channel_writer);
  if (error) goto exit;

  error = ockam_write(p_channel_writer, (uint8_t*) PING, PING_SIZE);
  if (error) goto exit;

  error = ockam_read(p_channel_reader, recv_buffer, MAX_DNS_NAME_LENGTH, &bytes_received);
  if (error) goto exit;
  if (0 != memcmp(recv_buffer, ACK, ACK_SIZE)) {
    error = OCKAM_ERROR_INTERFACE_CHANNEL;
    goto exit;
  }

exit:
  if (error) log_error(error, __func__);
  if (NULL != p_channel) ockam_channel_deinit(p_channel);
  if (NULL != p_transport) ockam_transport_deinit(p_transport);
  return error;
}
