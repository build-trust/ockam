
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

extern ockam_error_t
channel_write_to_app(void* ctx, uint8_t* p_clear_text, size_t clear_text_length, int32_t remote_app_address);

ockam_ip_address_t ockam_ip = { "", "127.0.0.1", 8000 };

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
exit:
  return error;
}

ockam_error_t establish_initiator_transport(ockam_transport_t*   p_transport,
                                            ockam_memory_t*      p_memory,
                                            codec_udp_address_t* local_ip,
                                            codec_udp_address_t* remote_ip,
                                            ockam_reader_t**     pp_reader,
                                            ockam_writer_t**     pp_writer)
{
  printf("0\n");
  ockam_error_t                       error = ockam_channel_interface_error_none;
  ockam_transport_socket_attributes_t xport_attrs;
  memset(&xport_attrs, 0, sizeof(xport_attrs));
  ockam_ip_address_t* local_ip_addr  = &xport_attrs.local_address;
  ockam_ip_address_t* remote_ip_addr = &xport_attrs.remote_address;

  printf("in %s\n", __func__);
  memset(&xport_attrs, 0, sizeof(xport_attrs));
  sprintf((char*) local_ip_addr->ip_address,
          "%d.%d.%d.%d",
          local_ip->host_address.ip_address.ipv4[0],
          local_ip->host_address.ip_address.ipv4[1],
          local_ip->host_address.ip_address.ipv4[2],
          local_ip->host_address.ip_address.ipv4[3]);
  local_ip_addr->port = local_ip->port;

  sprintf((char*) remote_ip_addr->ip_address,
          "%d.%d.%d.%d",
          remote_ip->host_address.ip_address.ipv4[0],
          remote_ip->host_address.ip_address.ipv4[1],
          remote_ip->host_address.ip_address.ipv4[2],
          remote_ip->host_address.ip_address.ipv4[3]);
  remote_ip_addr->port = remote_ip->port;

  xport_attrs.p_memory = p_memory;

  error = ockam_transport_socket_udp_init(p_transport, &xport_attrs);
  if (ockam_error_has_error(&error)) goto exit;

  error = ockam_transport_connect(p_transport, pp_reader, pp_writer, 10, 1);
  if (ockam_error_has_error(&error)) goto exit;

exit:
  if (ockam_error_has_error(&error)) ockam_log_error("%s: %d", error.domain, error.code);
  return error;
}

ockam_channel_t channel = { 0 };

ockam_error_t c_elixir_channel_initiator(ockam_vault_t*   vault,
                                         ockam_memory_t*  p_memory,
                                         codec_address_t* local_host_address,
                                         codec_address_t* local_address,
                                         codec_route_t*   onward_route)
{
  ockam_transport_t          transport = { 0 };
  ockam_reader_t*            p_channel_reader;
  ockam_writer_t*            p_channel_writer;
  ockam_reader_t*            p_transport_reader;
  ockam_writer_t*            p_transport_writer;
  uint8_t                    recv_buffer[MAX_XX_TRANSMIT_SIZE];
  size_t                     bytes_received = 0;
  ockam_channel_attributes_t channel_attrs;

  ockam_error_t  error = ockam_channel_interface_error_none;

  printf("in %s\n", __func__);
  printf("onward_route->count_addresses: %d\n", onward_route->count_addresses);
  if ((onward_route->count_addresses < 1) || (onward_route->p_addresses[0].type != ADDRESS_UDP)) {
    printf("1\n");
    error.code = -1;
    goto exit;
  }

  printf("calling establish_initiator_transport\n");
  error = establish_initiator_transport(&transport,
                                        p_memory,
                                        &local_host_address->address.socket_address.udp_address,
                                        &onward_route->p_addresses[0].address.socket_address.udp_address,
                                        &p_transport_reader,
                                        &p_transport_writer);
  if (ockam_error_has_error(&error)) goto exit;

  channel_attrs.reader                = p_transport_reader;
  channel_attrs.writer                = p_transport_writer;
  channel_attrs.memory                = p_memory;
  channel_attrs.vault                 = vault;
  channel_attrs.route.count_addresses = onward_route->count_addresses;
  channel_attrs.route.p_addresses     = channel_attrs.route_addresses;
  memcpy(
    channel_attrs.route_addresses, onward_route->p_addresses, onward_route->count_addresses * sizeof(codec_address_t));
  memcpy(&channel_attrs.local_host_address, local_host_address, sizeof(channel_attrs.local_host_address));

  error = ockam_channel_init(&channel, &channel_attrs);
  if (ockam_error_has_error(&error)) goto exit;

  error = ockam_channel_connect(&channel, onward_route, &p_channel_reader, &p_channel_writer);
  if (ockam_error_has_error(&error)) goto exit;

  char                        a[80];
  size_t                      line_length           = 0;
  size_t                      line_size             = sizeof(a);
  char*                       p_line                = a;
  uint32_t                    responder_app_address = 0;
  ockam_channel_poll_result_t result;

  p_line = a;
  printf("Enter text to send: \n");
  line_length             = getline(&p_line, &line_size, stdin);
  p_line[line_length - 1] = 0;
  error                   = channel_write_to_app(&channel, (uint8_t*) p_line, line_length, responder_app_address);
  if (ockam_error_has_error(&error)) goto exit;

  error = ockam_read(p_channel_reader, recv_buffer, MAX_XX_TRANSMIT_SIZE, &bytes_received);
  if ((error.code == OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA
       && OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN == error.domain)) {
    do {
      error = ockam_channel_poll(&channel, &result);
      if (ockam_error_has_error(&error) && !(error.code == OCKAM_TRANSPORT_INTERFACE_ERROR_NO_DATA
                                              && OCKAM_TRANSPORT_INTERFACE_ERROR_DOMAIN == error.domain)) goto exit;
      usleep(500 * 1000);
    } while (ockam_error_has_error(&error));
  }
  printf(" Received %ld bytes: %s\n", result.bytes_read, result.read_buffer);

exit:
  if (ockam_error_has_error(&error)) {
    ockam_log_error("%s: %d", error.domain, error.code);
    ockam_channel_deinit(&channel);
    if (NULL != transport.ctx) ockam_transport_deinit(&transport);
  }

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
  int     status            = 0;
  int32_t responder_process = 0;

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
  printf("route addresses %d\n", route.count_addresses);
  for (int i = 0; i < route.count_addresses; ++i) {
    if (route_addresses[i].type == ADDRESS_LOCAL) {
      printf("route_addresses[%d]: %d %d %.8x\n",
             i,
             route_addresses[1].type,
             route_addresses[1].address.local_address.size,
             *(uint32_t*) (route_addresses[1].address.local_address.address));
    } else if (route_addresses[i].type == ADDRESS_UDP) {
      printf("route_addresses[%d]: %d.%d.%d.%d:%u\n",
             i,
             route_addresses[i].address.socket_address.udp_address.host_address.ip_address.ipv4[0],
             route_addresses[i].address.socket_address.udp_address.host_address.ip_address.ipv4[1],
             route_addresses[i].address.socket_address.udp_address.host_address.ip_address.ipv4[2],
             route_addresses[i].address.socket_address.udp_address.host_address.ip_address.ipv4[3],
             route_addresses[i].address.socket_address.udp_address.port);
    }
  }
  char a[80];
  printf("Initiator IP Address     : %d.%d.%d.%d:%u\n",
         initiator_ip_address.address.socket_address.udp_address.host_address.ip_address.ipv4[0],
         initiator_ip_address.address.socket_address.udp_address.host_address.ip_address.ipv4[1],
         initiator_ip_address.address.socket_address.udp_address.host_address.ip_address.ipv4[2],
         initiator_ip_address.address.socket_address.udp_address.host_address.ip_address.ipv4[3],
         initiator_ip_address.address.socket_address.udp_address.port);

  error = c_elixir_channel_initiator(&vault, &memory, &initiator_ip_address, &initiator_local_address, &route);
  if (ockam_error_has_error(&error)) goto exit;

exit:
  return error.code + status;
}
