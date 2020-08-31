#include <stdio.h>
#include <string.h>
#include <unistd.h>

#include "ockam/error.h"
#include "ockam/memory.h"
#include "ockam/log.h"
#include "ockam/transport.h"
#include "ockam/channel.h"
#include "channel_test.h"

size_t ip_string_to_octets(char* str, uint8_t* octets, size_t octets_size)
{
  char*  token        = NULL;
  size_t count_octets = 0;
  char*  save_ptr     = NULL;

  token = strtok_r(str, ".", &save_ptr);
  while ((token) && (count_octets <= octets_size)) {
    *octets++ = atoi(token);
    ++count_octets;
    token = strtok_r(NULL, ".", &save_ptr);
  }

  return count_octets;
}

ockam_error_t
read_route_configuration(char* filename, codec_route_t* route, codec_address_t* initiator, codec_address_t* responder)
{
  ockam_error_t error = ockam_channel_interface_error_none;
  error.code          = -1;
  char          line[80];
  size_t        line_size   = 80;
  char*         p_line      = line;
  char*         token       = NULL;
  size_t        line_length = 0;
  char          file_path[128];
  char*         save_ptr;

  getcwd(file_path, 128);
  strcat(file_path, "/");
  strcat(file_path, filename);

  FILE* fp = fopen(file_path, "r");
  if (0 == fp) goto exit;

  do {
    line_length = getline(&p_line, &line_size, fp);
    if ((int32_t) line_length > 0) {
      token = strtok_r(p_line, ":", &save_ptr);
      while ((token) && (token[0] != '\n')) {
        switch (token[0]) {
        case '#':
          token = NULL;
          break;
        case 'r':
          token = strtok_r(NULL, ":", &save_ptr);
          if ('l' == token[0]) {
            responder->type                       = ADDRESS_LOCAL;
            token                                 = strtok_r(NULL, ":", &save_ptr);
            responder->address.local_address.size = atoi(token);
            token                                 = strtok_r(NULL, ":", &save_ptr);
            strcpy((char*) responder->address.local_address.address, token);
          } else {
            int count_octets                                                = 0;
            responder->type                                                 = ADDRESS_UDP;
            responder->address.socket_address.udp_address.host_address.type = HOST_ADDRESS_IPV4;
            count_octets =
              ip_string_to_octets(token, responder->address.socket_address.udp_address.host_address.ip_address.ipv4, 4);
            if (4 != count_octets) goto exit;
            token                                              = strtok_r(NULL, ":", &save_ptr);
            responder->address.socket_address.udp_address.port = atoi(token);
          }
          token = NULL;
          break;
        case 'i':
          token = strtok_r(NULL, ":", &save_ptr);
          if ('l' == token[0]) {
            //            initiator_local_address.type = ADDRESS_LOCAL;
            //            token = strtok_r(NULL, ":", &save_ptr);
            //            initiator_local_address.address.local_address.size = atoi(token);
            //            token = strtok_r(NULL, ":", &save_ptr);
            //            strcpy((char*)initiator_local_address.address.local_address.address, token);
          } else {
            int count_octets                                                = 0;
            initiator->type                                                 = ADDRESS_UDP;
            initiator->address.socket_address.udp_address.host_address.type = HOST_ADDRESS_IPV4;
            count_octets =
              ip_string_to_octets(token, initiator->address.socket_address.udp_address.host_address.ip_address.ipv4, 4);
            if (4 != count_octets) goto exit;
            token                                              = strtok_r(NULL, ":", &save_ptr);
            initiator->address.socket_address.udp_address.port = atoi(token);
          }
          token = NULL;
          break;
        case '0':
        case '1':
        case '2':
        case '3':
        case '4': {
          int idx = atoi(&token[0]);
          route->count_addresses++;
          token = strtok_r(NULL, ":", &save_ptr);
          if (token[0] == 'l') {
            size_t  size = 0;
            int32_t address;
            route->p_addresses[idx].type = ADDRESS_LOCAL;
            token                        = strtok_r(NULL, ":", &save_ptr);
            string_to_hex((uint8_t*) token, (uint8_t*) &address, &size);
            strncpy((char*) route->p_addresses[idx].address.local_address.address, (char*) &address, sizeof(address));
            route->p_addresses[idx].address.local_address.size = size;
          } else {
            int count_octets                                                             = 0;
            route->p_addresses[idx].type                                                 = ADDRESS_UDP;
            route->p_addresses[idx].address.socket_address.udp_address.host_address.type = HOST_ADDRESS_IPV4;
            count_octets                                                                 = ip_string_to_octets(
              token, route->p_addresses[idx].address.socket_address.udp_address.host_address.ip_address.ipv4, 4);
            if (4 != count_octets) goto exit;
            token                                                           = strtok_r(NULL, ":", &save_ptr);
            route->p_addresses[idx].address.socket_address.udp_address.port = atoi(token);
          }
          token = NULL;
          break;
        }
        default:
          goto exit;
        }
      }
    }
  } while ((int32_t) line_length > 0);

  error = ockam_channel_interface_error_none;

exit:
  return error;
}
