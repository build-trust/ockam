
#include <stdio.h>
#include <string.h>
#include <stdbool.h>

#include "ockam/error.h"
#include "ockam/key_agreement.h"
#include "ockam/memory.h"
#include "ockam/syslog.h"
#include "ockam/transport.h"
#include "ockam/vault.h"
#include "ockam/channel.h"
#include "channel_test.h"

ockam_error_t establish_responder_transport(ockam_transport_t*  p_transport,
                                            ockam_memory_t*     p_memory,
                                            ockam_ip_address_t* p_address,
                                            ockam_reader_t**    pp_reader,
                                            ockam_writer_t**    pp_writer)
{
  ockam_error_t                       error = OCKAM_ERROR_NONE;
  ockam_transport_socket_attributes_t tcp_attributes;

  memset(&tcp_attributes, 0, sizeof(tcp_attributes));
  memcpy(&tcp_attributes.listen_address, p_address, sizeof(ockam_ip_address_t));
  tcp_attributes.p_memory = p_memory;
  error                   = ockam_transport_socket_tcp_init(p_transport, &tcp_attributes);
  if (error) goto exit;

  // Wait for a connection
  error = ockam_transport_accept(p_transport, pp_reader, pp_writer, NULL);
  if (error) goto exit;

  error = OCKAM_ERROR_NONE;

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t channel_responder(ockam_vault_t* vault, ockam_memory_t* p_memory, ockam_ip_address_t* ip_address)
{
  ockam_error_t              error     = OCKAM_ERROR_NONE;
  ockam_transport_t          transport = { 0 };
  ockam_channel_t            channel   = { 0 };
  ockam_reader_t*            p_ch_reader;
  ockam_writer_t*            p_ch_writer;
  ockam_reader_t*            p_transport_reader;
  ockam_writer_t*            p_transport_writer;
  uint8_t                    send_buffer[MAX_XX_TRANSMIT_SIZE];
  uint8_t                    recv_buffer[MAX_XX_TRANSMIT_SIZE];
  size_t                     bytes_received = 0;
  size_t                     transmit_size  = 0;
  ockam_channel_attributes_t channel_attrs;

  error = establish_responder_transport(&transport, p_memory, ip_address, &p_transport_reader, &p_transport_writer);
  if (error) goto exit;

  channel_attrs.reader = p_transport_reader;
  channel_attrs.writer = p_transport_writer;
  channel_attrs.memory = p_memory;
  channel_attrs.vault  = vault;

  error = ockam_channel_init(&channel, &channel_attrs);
  if (error) goto exit;

  error = ockam_channel_accept(&channel, &p_ch_reader, &p_ch_writer);
  if (error) goto exit;

  error = ockam_read(p_ch_reader, recv_buffer, MAX_DNS_NAME_LENGTH, &bytes_received);
  if (error) goto exit;
  if (0 != memcmp(recv_buffer, PING, PING_SIZE)) {
    error = OCKAM_ERROR_INTERFACE_CHANNEL;
    goto exit;
  }

  error = ockam_write(p_ch_writer, (uint8_t*) ACK, ACK_SIZE);
  if (error) goto exit;

  printf("Responder received %ld bytes: %s\n", bytes_received, recv_buffer);

exit:
  if (error) log_error(error, __func__);
  ockam_channel_deinit(&channel);
  ockam_transport_deinit(&transport);
  return error;
}
