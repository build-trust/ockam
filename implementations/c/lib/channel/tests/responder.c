
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

ockam_error_t establish_responder_transport(ockam_transport_t** pp_transport,
                                            ockam_ip_address_t* p_address,
                                            ockam_reader_t**    pp_reader,
                                            ockam_writer_t**    pp_writer)
{
  ockam_error_t                           error = OCKAM_ERROR_NONE;
  ockam_transport_tcp_socket_attributes_t tcp_attributes;

  memset(&tcp_attributes, 0, sizeof(tcp_attributes));
  memcpy(&tcp_attributes.listen_address, p_address, sizeof(ockam_ip_address_t));
  error = ockam_transport_socket_tcp_init(pp_transport, &tcp_attributes);
  if (error) goto exit;

  // Wait for a connection
  error = ockam_transport_accept(*pp_transport, pp_reader, pp_writer, NULL);
  if (error) goto exit;

  error = OCKAM_ERROR_NONE;

exit:
  if (error) log_error(error, __func__);
  return error;
}

ockam_error_t channel_responder(ockam_vault_t* vault, ockam_memory_t* p_memory, ockam_ip_address_t* ip_address)
{
  ockam_error_t              error       = OCKAM_ERROR_NONE;
  ockam_transport_t*         p_transport = NULL;
  ockam_channel_t*           p_channel   = NULL;
  ockam_reader_t*            p_ch_reader;
  ockam_writer_t*            p_ch_writer;
  key_establishment_xx       handshake;
  uint8_t                    send_buffer[MAX_TRANSMIT_SIZE];
  uint8_t                    recv_buffer[MAX_TRANSMIT_SIZE];
  size_t                     bytes_received = 0;
  size_t                     transmit_size  = 0;
  ockam_channel_attributes_t channel_attrs;

  memset(&handshake, 0, sizeof(handshake));
  handshake.vault = vault;

  error = establish_responder_transport(&p_transport, ip_address, &handshake.p_reader, &handshake.p_writer);
  if (error) goto exit;

  channel_attrs.reader = handshake.p_reader;
  channel_attrs.writer = handshake.p_writer;
  channel_attrs.memory = p_memory;
  channel_attrs.vault  = vault;

  error = ockam_channel_init(&p_channel, &channel_attrs);
  if (error) goto exit;

  error = ockam_channel_accept(p_channel, &p_ch_reader, &p_ch_writer);
  if (error) goto exit;

  error = ockam_read(p_ch_reader, recv_buffer, MAX_DNS_NAME_LENGTH, &bytes_received);
  if (0 != memcmp(recv_buffer, PING, PING_SIZE)) {
    error = OCKAM_ERROR_INTERFACE_CHANNEL;
    goto exit;
  }

  error = ockam_write(p_ch_writer, (uint8_t*) ACK, ACK_SIZE);
  if (error) goto exit;

  printf("Responder received %ld bytes: %s\n", bytes_received, recv_buffer);

exit:
  if (error) log_error(error, __func__);
  if (NULL != p_channel) ockam_channel_deinit(p_channel);
  if (NULL != p_transport) ockam_transport_deinit(p_transport);
  return error;
}
