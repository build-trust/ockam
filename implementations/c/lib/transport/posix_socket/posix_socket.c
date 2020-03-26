#include "posix_socket.h"

#include <stdio.h>
#include <stdlib.h>

#include "ockam/error.h"
#include "ockam/syslog.h"

/**
 * MakeSocketAddress - constructs network-ready socket address from
 * user-friendly format
 * @param p_ip_address - (in) pointer to IP address string in nnn.nnn.nnn format
 * @param port  - (in) port number in local machine byte order
 * @param p_socketAddress - (out) network-ready sockaddr_in structure
 * @return - kErrorNone if successful
 */
TransportError MakeSocketAddress(char *p_ip_address, in_port_t port, struct sockaddr_in *p_socketAddress) {
  TransportError status = kErrorNone;
  int in_status = 0;

  // Get the host IP address and port
  p_socketAddress->sin_family = AF_INET;
  p_socketAddress->sin_port = htons(port);
  if (NULL != p_ip_address) {
    in_status = inet_pton(AF_INET, p_ip_address, &p_socketAddress->sin_addr.s_addr);
    if (1 != in_status) {
      log_error(status, "inet_pton failed in MakeSocketAddress");
      status = kBadAddress;
      goto exit_block;
    }
  } else {
    p_socketAddress->sin_addr.s_addr = htonl(INADDR_ANY);
  }

exit_block:
  return status;
}
