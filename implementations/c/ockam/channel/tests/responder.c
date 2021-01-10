
#include <stdio.h>
#include <string.h>
#include <unistd.h>

#include "ockam/error.h"
#include "ockam/memory.h"
#include "ockam/memory/stdlib.h"
#include "ockam/log.h"
#include "ockam/transport.h"
#include "ockam/transport/socket_udp.h"
#include "ockam/random/urandom.h"
#include "ockam/vault.h"
#include "ockam/vault/default.h"
#include "ockam/channel.h"
#include "channel_test.h"

ockam_ip_address_t ockam_ip = { "", "127.0.0.1", 8000 };

extern ockam_error_t
channel_write_to_app(void* ctx, uint8_t* p_clear_text, size_t clear_text_length, int32_t remote_app_address);

void usage()
{
  printf("Usage\n");
  printf("  -f<filename>\t\t\tRead configuration from <filename>\n");
}

int parse_opts(int argc, char* argv[], char* filename)
{
  int ch;
  int error = 0;
  while ((ch = getopt(argc, argv, "hf:")) != -1) {
    switch (ch) {
    case 'f':
      strcpy(filename, optarg);
      break;
    default:
      usage();
      error = -1;
      break;
    }
  }
  return error;
}

ockam_error_t establish_responder_transport(ockam_transport_t*   p_transport,
                                            ockam_memory_t*      p_memory,
                                            codec_udp_address_t* local_ip,
                                            ockam_reader_t**     pp_reader,
                                            ockam_writer_t**     pp_writer)
{
  ockam_error_t                       error = ockam_channel_interface_error_none;
  ockam_transport_socket_attributes_t xport_attrs;
  ockam_ip_address_t*                 local_ip_addr = &xport_attrs.local_address;

  memset(&xport_attrs, 0, sizeof(xport_attrs));
  memcpy(&xport_attrs.local_address, local_ip, sizeof(ockam_ip_address_t));
  sprintf((char*) local_ip_addr->ip_address,
          "%d.%d.%d.%d",
          local_ip->host_address.ip_address.ipv4[0],
          local_ip->host_address.ip_address.ipv4[1],
          local_ip->host_address.ip_address.ipv4[2],
          local_ip->host_address.ip_address.ipv4[3]);
  local_ip_addr->port = local_ip->port;

  xport_attrs.p_memory = p_memory;
  error                = ockam_transport_socket_udp_init(p_transport, &xport_attrs);
  if (ockam_error_has_error(&error)) goto exit;

  // Wait for a connection
  error = ockam_transport_accept(p_transport, pp_reader, pp_writer, NULL);
  if (ockam_error_has_error(&error)) goto exit;

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_error_t c_elixir_channel_responder(ockam_vault_t*   vault,
                                         ockam_memory_t*  p_memory,
                                         codec_address_t* local_host_address,
                                         codec_address_t* local_address)
{
  ockam_error_t               error     = ockam_channel_interface_error_none;
  ockam_transport_t           transport = { 0 };
  ockam_channel_t             channel   = { 0 };
  ockam_reader_t*             p_ch_reader;
  ockam_writer_t*             p_ch_writer;
  ockam_reader_t*             p_transport_reader;
  ockam_writer_t*             p_transport_writer;
  ockam_channel_poll_result_t result = { 0 };
  uint8_t                     send_buffer[MAX_XX_TRANSMIT_SIZE];
  uint8_t                     recv_buffer[MAX_XX_TRANSMIT_SIZE];
  size_t                      bytes_received = 0;
  size_t                      transmit_size  = 0;
  ockam_channel_attributes_t  channel_attrs;
  struct ockam_ip_address_t   ip_address;

  sprintf((char*) ip_address.ip_address,
          "%d.%d.%d.%d",
          local_host_address->address.socket_address.udp_address.host_address.ip_address.ipv4[0],
          local_host_address->address.socket_address.udp_address.host_address.ip_address.ipv4[1],
          local_host_address->address.socket_address.udp_address.host_address.ip_address.ipv4[2],
          local_host_address->address.socket_address.udp_address.host_address.ip_address.ipv4[3]);
  ip_address.port = local_host_address->address.socket_address.udp_address.port;
  error           = establish_responder_transport(&transport,
                                        p_memory,
                                        &local_host_address->address.socket_address.udp_address,
                                        &p_transport_reader,
                                        &p_transport_writer);
  if (ockam_error_has_error(&error)) goto exit;

  channel_attrs.reader = p_transport_reader;
  channel_attrs.writer = p_transport_writer;
  channel_attrs.memory = p_memory;
  channel_attrs.vault  = vault;

  memcpy(&channel_attrs.local_host_address, local_host_address, sizeof(codec_address_t));
  error = ockam_channel_init(&channel, &channel_attrs);
  if (ockam_error_has_error(&error)) goto exit;

  error = ockam_channel_accept(&channel, &p_ch_reader, &p_ch_writer);
  if (ockam_error_has_error(&error) && !(error.code == OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA
                                         && OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN == error.domain)) goto exit;

  do {
    error = ockam_channel_poll(&channel, &result);
    if (ockam_error_has_error(&error) && !(error.code == OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA
                                           && OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN == error.domain)) goto exit;
    usleep(500 * 1000);
  } while (!result.channel_is_secure);

  error = ockam_read(p_ch_reader, recv_buffer, MAX_XX_TRANSMIT_SIZE, &bytes_received);
  if (error.code == OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA
      && OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN == error.domain) {
    do {
      error = ockam_channel_poll(&channel, &result);
      if (ockam_error_has_error(&error)) {
        if (!(error.code == OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA
              && OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN == error.domain)) goto exit;
      }
      usleep(500 * 1000);
    } while (ockam_error_has_error(&error));
  }
  printf(" Received %ld bytes: %s\n", result.bytes_read, result.read_buffer);

  char     a[80];
  size_t   line_length           = 0;
  size_t   line_size             = sizeof(a);
  char*    p_line                = a;
  uint32_t responder_app_address = 0;

  p_line = a;
  printf("Enter text to send: \n");
  line_length             = getline(&p_line, &line_size, stdin);
  p_line[line_length - 1] = 0;
  error                   = channel_write_to_app(&channel, (uint8_t*) p_line, line_length, responder_app_address);
  if (ockam_error_has_error(&error)) goto exit;

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  ockam_channel_deinit(&channel);
  ockam_transport_deinit(&transport);
  return error;
}

int main(int argc, char* argv[])
{
  ockam_error_t                    error                   = ockam_channel_interface_error_none;
  ockam_vault_t                    vault                   = { 0 };
  ockam_memory_t                   memory                  = { 0 };
  ockam_random_t                   random                  = { 0 };
  ockam_vault_default_attributes_t vault_attributes        = { .memory = &memory, .random = &random };
  codec_route_t                    route                   = { 0 };
  codec_address_t                  route_addresses[5]      = { { 0 }, { 0 }, { 0 }, { 0 }, { 0 } };
  codec_address_t                  initiator_ip_address    = { 0 };
  codec_address_t                  responder_ip_address    = { 0 };
  codec_address_t                  initiator_local_address = { 0 };
  codec_address_t                  responder_local_address = { 0 };

  char    filename[128];
  int     responder_status  = 0;
  int     initiator_status  = 0;
  int     fork_status       = 0;
  int32_t responder_process = 0;
  int     status            = 0;

  error = ockam_memory_stdlib_init(&memory);
  if (ockam_error_has_error(&error)) goto exit;

  error = ockam_random_urandom_init(&random);
  if (ockam_error_has_error(&error)) goto exit;

  error = ockam_vault_default_init(&vault, &vault_attributes);
  if (ockam_error_has_error(&error)) goto exit;

  /*-------------------------------------------------------------------------
   * Parse options
   *-----------------------------------------------------------------------*/
  route.p_addresses = route_addresses;
  status            = parse_opts(argc, argv, filename);
  if (status) goto exit;
  error = read_route_configuration(filename, &route, &initiator_ip_address, &responder_ip_address);
  if (ockam_error_has_error(&error)) goto exit;

  error = c_elixir_channel_responder(&vault, &memory, &responder_ip_address, NULL);

  //  size_t line_length = 0;
  //  size_t line_size = sizeof(a);
  //  char* p_line = a;
  //  printf("Channel secured!\n");
  //  printf("Enter local address for responder: \n");
  //  line_length = getline(&p_line, &line_size, stdin);
  //  uint32_t responder_app_address;
  //  size_t bytes;
  //  string_to_hex((uint8_t*)a, (uint8_t*)&responder_app_address, &bytes);

  //  do {
  //    p_line = a;
  //    printf("Enter text to send: \n");
  //    line_length = getline(&p_line, &line_size, stdin);
  //    error = channel_write_to_app(&channel, (uint8_t*)p_line, line_length-1, responder_app_address);
  //    if(error) goto exit;
  //  } while (a[0] != 'q');

exit:
  return error.code + status;
}
